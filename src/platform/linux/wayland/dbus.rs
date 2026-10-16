//! Minimal blocking D-Bus session-bus client.
//!
//! Only what `rsautogui` needs on Wayland: `Hello`, `AddMatch`, method calls
//! with basic signatures (`s`, `o`, `b`, `i`, `u`, `a{sv}`, structs,
//! variants) and waiting for portal `Response` signals. No fd passing.

use std::io::{self, Read, Write};
use std::os::linux::net::SocketAddrExt;
use std::os::unix::net::UnixStream;
use std::time::{Duration, Instant};

extern "C" {
    fn getuid() -> u32;
}

// Message types / header field codes.
const TYPE_METHOD_CALL: u8 = 1;
const TYPE_METHOD_RETURN: u8 = 2;
const TYPE_ERROR: u8 = 3;
const TYPE_SIGNAL: u8 = 4;

const F_PATH: u8 = 1;
const F_INTERFACE: u8 = 2;
const F_MEMBER: u8 = 3;
const F_ERROR_NAME: u8 = 4;
const F_REPLY_SERIAL: u8 = 5;
const F_DEST: u8 = 6;
const F_SIGNATURE: u8 = 8;

// ---------------------------------------------------------------------------
// Value model
// ---------------------------------------------------------------------------

// Some variants only exist for protocol completeness (replies we parse but
// whose values this crate never inspects).
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) enum DVal {
    Byte(u8),
    Bool(bool),
    I32(i32),
    U32(u32),
    Str(String),
    ObjPath(String),
    Array(Vec<DVal>),
    Struct(Vec<DVal>),
    Variant(Box<DVal>),
}

impl DVal {
    pub(crate) fn as_str(&self) -> Option<&str> {
        match self {
            DVal::Str(s) => Some(s),
            DVal::ObjPath(s) => Some(s),
            _ => None,
        }
    }
    pub(crate) fn as_bool(&self) -> Option<bool> {
        match self {
            DVal::Bool(b) => Some(*b),
            _ => None,
        }
    }
    pub(crate) fn as_u32(&self) -> Option<u32> {
        match self {
            DVal::U32(u) => Some(*u),
            _ => None,
        }
    }
    pub(crate) fn as_array(&self) -> Option<&[DVal]> {
        match self {
            DVal::Array(a) => Some(a),
            _ => None,
        }
    }
    pub(crate) fn variant_inner(&self) -> Option<&DVal> {
        match self {
            DVal::Variant(v) => Some(v),
            _ => None,
        }
    }
    /// Looks up `key` in a `a{sv}`-style array of `{string, variant}` structs.
    pub(crate) fn dict_get(&self, key: &str) -> Option<&DVal> {
        let arr = self.as_array()?;
        for e in arr {
            if let DVal::Struct(kv) = e {
                if kv.len() == 2 {
                    if let DVal::Str(k) = &kv[0] {
                        if k == key {
                            return Some(&kv[1]);
                        }
                    }
                }
            }
        }
        None
    }
}

// ---------------------------------------------------------------------------
// Marshaling writer
// ---------------------------------------------------------------------------

pub(crate) struct Writer {
    buf: Vec<u8>,
}

impl Writer {
    pub(crate) fn new() -> Self {
        Self { buf: Vec::new() }
    }
    pub(crate) fn into_bytes(self) -> Vec<u8> {
        self.buf
    }
    fn align(&mut self, n: usize) {
        while self.buf.len() % n != 0 {
            self.buf.push(0);
        }
    }
    pub(crate) fn u8_(&mut self, v: u8) {
        self.buf.push(v);
    }
    pub(crate) fn u32_(&mut self, v: u32) {
        self.align(4);
        self.buf.extend_from_slice(&v.to_le_bytes());
    }
    pub(crate) fn i32_(&mut self, v: i32) {
        self.align(4);
        self.buf.extend_from_slice(&v.to_le_bytes());
    }
    pub(crate) fn bool_(&mut self, v: bool) {
        self.u32_(if v { 1 } else { 0 });
    }
    pub(crate) fn string(&mut self, s: &str) {
        self.align(4);
        self.buf.extend_from_slice(&(s.len() as u32).to_le_bytes());
        self.buf.extend_from_slice(s.as_bytes());
        self.buf.push(0);
    }
    pub(crate) fn path(&mut self, s: &str) {
        self.string(s);
    }
    fn sig_(&mut self, s: &str) {
        assert!(s.len() <= 255, "D-Bus signature too long");
        self.buf.push(s.len() as u8);
        self.buf.extend_from_slice(s.as_bytes());
        self.buf.push(0);
    }
    pub(crate) fn struct_(&mut self, f: impl FnOnce(&mut Writer)) {
        self.align(8);
        f(self);
    }
    pub(crate) fn dict_entry(&mut self, f: impl FnOnce(&mut Writer)) {
        self.struct_(f);
    }
    pub(crate) fn array(&mut self, elem_align: usize, f: impl FnOnce(&mut Writer)) {
        self.align(4);
        let len_pos = self.buf.len();
        self.buf.extend_from_slice(&[0, 0, 0, 0]);
        self.align(elem_align);
        let start = self.buf.len();
        f(self);
        let len = (self.buf.len() - start) as u32;
        self.buf[len_pos..len_pos + 4].copy_from_slice(&len.to_le_bytes());
    }
    pub(crate) fn variant(&mut self, sig: &str, f: impl FnOnce(&mut Writer)) {
        self.sig_(sig);
        f(self);
    }
}

// ---------------------------------------------------------------------------
// Unmarshaling reader
// ---------------------------------------------------------------------------

struct Reader<'a> {
    b: &'a [u8],
    off: usize,
}

fn err(msg: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg)
}

impl<'a> Reader<'a> {
    fn need(&self, n: usize) -> io::Result<()> {
        if self.off + n > self.b.len() {
            return Err(err(format!(
                "D-Bus message truncated (need {} bytes at offset {})",
                n, self.off
            )));
        }
        Ok(())
    }
    fn align(&mut self, n: usize) {
        while self.off % n != 0 {
            self.off += 1;
        }
    }
    fn u8_(&mut self) -> io::Result<u8> {
        self.need(1)?;
        let v = self.b[self.off];
        self.off += 1;
        Ok(v)
    }
    fn u32_(&mut self) -> io::Result<u32> {
        self.align(4);
        self.need(4)?;
        let mut t = [0u8; 4];
        t.copy_from_slice(&self.b[self.off..self.off + 4]);
        self.off += 4;
        Ok(u32::from_le_bytes(t))
    }
    fn i32_(&mut self) -> io::Result<i32> {
        Ok(self.u32_()? as i32)
    }
    fn bool_(&mut self) -> io::Result<bool> {
        Ok(self.u32_()? != 0)
    }
    fn string(&mut self) -> io::Result<String> {
        let len = self.u32_()? as usize;
        self.need(len + 1)?;
        let s = std::str::from_utf8(&self.b[self.off..self.off + len])
            .map_err(|e| err(format!("invalid D-Bus string: {e}")))?
            .to_owned();
        self.off += len;
        let nul = self.u8_()?;
        if nul != 0 {
            return Err(err("D-Bus string not NUL-terminated".to_string()));
        }
        Ok(s)
    }
    fn sig_(&mut self) -> io::Result<String> {
        let len = self.u8_()? as usize;
        self.need(len + 1)?;
        let s = std::str::from_utf8(&self.b[self.off..self.off + len])
            .map_err(|e| err(format!("invalid D-Bus signature: {e}")))?
            .to_owned();
        self.off += len + 1; // includes NUL
        Ok(s)
    }
}

/// Parses one value of the type at `sig[pos]` (advancing `pos`).
fn parse_value(sig: &[u8], pos: &mut usize, r: &mut Reader) -> io::Result<DVal> {
    if *pos >= sig.len() {
        return Err(err("D-Bus signature exhausted".to_string()));
    }
    let c = sig[*pos];
    *pos += 1;
    match c {
        b'y' => Ok(DVal::Byte(r.u8_()?)),
        b'b' => Ok(DVal::Bool(r.bool_()?)),
        b'i' => Ok(DVal::I32(r.i32_()?)),
        b'u' => Ok(DVal::U32(r.u32_()?)),
        b's' => Ok(DVal::Str(r.string()?)),
        b'o' => Ok(DVal::ObjPath(r.string()?)),
        b'g' => Ok(DVal::Str(r.sig_()?)),
        b'v' => {
            let inner = r.sig_()?;
            let ib = inner.as_bytes();
            let mut ip = 0;
            let v = parse_value(ib, &mut ip, r)?;
            if ip != ib.len() {
                return Err(err("trailing bytes in D-Bus variant signature".to_string()));
            }
            Ok(DVal::Variant(Box::new(v)))
        }
        b'a' => {
            if *pos >= sig.len() {
                return Err(err("array without element type".to_string()));
            }
            // Find the end of the element type within the signature.
            let mut end_pos = *pos;
            skip_type(sig, &mut end_pos)?;
            let align = elem_align(sig, *pos);
            let alen = r.u32_()? as usize;
            r.align(align);
            let end = r.off + alen;
            let mut elems = Vec::new();
            let is_dict = sig[*pos] == b'{';
            while r.off < end {
                if is_dict {
                    r.align(8);
                    // `{key, value}`: key is a single basic type char.
                    let kt = sig[*pos + 1];
                    let k = parse_basic(kt, r)?;
                    let mut ep = *pos + 2;
                    let v = parse_value(sig, &mut ep, r)?;
                    elems.push(DVal::Struct(vec![k, v]));
                } else {
                    let mut ep = *pos;
                    elems.push(parse_value(sig, &mut ep, r)?);
                }
            }
            *pos = end_pos;
            Ok(DVal::Array(elems))
        }
        b'(' | b'r' => {
            r.align(8);
            let mut fields = Vec::new();
            loop {
                if *pos >= sig.len() {
                    return Err(err("unterminated D-Bus struct".to_string()));
                }
                if sig[*pos] == b')' {
                    *pos += 1;
                    break;
                }
                fields.push(parse_value(sig, pos, r)?);
            }
            Ok(DVal::Struct(fields))
        }
        _ => Err(err(format!("unsupported D-Bus type '{}'", c as char))),
    }
}

fn parse_basic(t: u8, r: &mut Reader) -> io::Result<DVal> {
    match t {
        b'y' => Ok(DVal::Byte(r.u8_()?)),
        b'b' => Ok(DVal::Bool(r.bool_()?)),
        b'i' => Ok(DVal::I32(r.i32_()?)),
        b'u' => Ok(DVal::U32(r.u32_()?)),
        b's' => Ok(DVal::Str(r.string()?)),
        b'o' => Ok(DVal::ObjPath(r.string()?)),
        b'g' => Ok(DVal::Str(r.sig_()?)),
        _ => Err(err("unsupported D-Bus dict key type".to_string())),
    }
}

fn elem_align(sig: &[u8], pos: usize) -> usize {
    if pos >= sig.len() {
        return 1;
    }
    match sig[pos] {
        b'y' | b'v' => 1,
        b'i' | b'u' | b's' | b'o' | b'b' | b'a' | b'g' => 4,
        b'(' | b'r' | b'{' => 8,
        _ => 1,
    }
}

/// Advances `pos` past one complete type in `sig` (no data is read).
fn skip_type(sig: &[u8], pos: &mut usize) -> io::Result<()> {
    if *pos >= sig.len() {
        return Err(err("D-Bus signature exhausted".to_string()));
    }
    let c = sig[*pos];
    *pos += 1;
    match c {
        b'a' => skip_type(sig, pos),
        b'(' | b'r' => {
            loop {
                if *pos >= sig.len() {
                    return Err(err("unterminated D-Bus struct".to_string()));
                }
                if sig[*pos] == b')' {
                    *pos += 1;
                    break;
                }
                skip_type(sig, pos)?;
            }
            Ok(())
        }
        b'{' => {
            // Key is a single basic type char.
            if *pos >= sig.len() {
                return Err(err("unterminated D-Bus dict entry".to_string()));
            }
            *pos += 1;
            skip_type(sig, pos)?;
            if *pos >= sig.len() || sig[*pos] != b'}' {
                return Err(err("unterminated D-Bus dict entry".to_string()));
            }
            *pos += 1;
            Ok(())
        }
        _ => Ok(()),
    }
}

fn parse_body(sig: &str, data: &[u8]) -> io::Result<Vec<DVal>> {
    let sb = sig.as_bytes();
    let mut pos = 0;
    let mut r = Reader { b: data, off: 0 };
    let mut out = Vec::new();
    while pos < sb.len() {
        out.push(parse_value(sb, &mut pos, &mut r)?);
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Messages
// ---------------------------------------------------------------------------

#[allow(dead_code)]
pub(crate) struct Message {
    pub mtype: u8,
    pub sender: String,
    pub path: String,
    pub iface: String,
    pub member: String,
    pub error_name: String,
    pub reply_serial: u32,
    pub body_sig: String,
    pub body: Vec<DVal>,
}

fn read_exact_timeout(s: &mut UnixStream, buf: &mut [u8]) -> io::Result<()> {
    let mut read = 0;
    while read < buf.len() {
        match s.read(&mut buf[read..]) {
            Ok(0) => return Err(err("D-Bus connection closed".to_string())),
            Ok(n) => read += n,
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

fn read_message(stream: &mut UnixStream) -> io::Result<Message> {
    let mut fixed = [0u8; 12];
    read_exact_timeout(stream, &mut fixed)?;
    if fixed[0] != b'l' {
        return Err(err("only little-endian D-Bus supported".to_string()));
    }
    let mtype = fixed[1];
    let version = fixed[3];
    if version != 1 {
        return Err(err("unsupported D-Bus protocol version".to_string()));
    }
    let body_len = u32::from_le_bytes(fixed[4..8].try_into().unwrap()) as usize;
    let _serial = u32::from_le_bytes(fixed[8..12].try_into().unwrap());

    // Fields array: u32 length + entries.
    let mut alen_b = [0u8; 4];
    read_exact_timeout(stream, &mut alen_b)?;
    let alen = u32::from_le_bytes(alen_b) as usize;
    let mut fields_raw = vec![0u8; alen];
    if alen > 0 {
        read_exact_timeout(stream, &mut fields_raw)?;
    }
    // Header pads to a multiple of 8 (12 + 4 + alen, padded).
    let used = 12 + 4 + alen;
    let pad = (8 - used % 8) % 8;
    if pad > 0 {
        let mut p = vec![0u8; pad];
        read_exact_timeout(stream, &mut p)?;
    }

    let sender = String::new();
    let mut path = String::new();
    let mut iface = String::new();
    let mut member = String::new();
    let mut error_name = String::new();
    let mut reply_serial = 0u32;
    let mut body_sig = String::new();
    if alen > 0 {
        let mut r = Reader {
            b: &fields_raw,
            off: 0,
        };
        while r.off < fields_raw.len() {
            r.align(8);
            if r.off >= fields_raw.len() {
                break;
            }
            let code = r.u8_()?;
            let vsig = r.sig_()?;
            match code {
                F_PATH => path = r.string()?,
                F_INTERFACE => iface = r.string()?,
                F_MEMBER => member = r.string()?,
                F_ERROR_NAME => error_name = r.string()?,
                F_REPLY_SERIAL => reply_serial = r.u32_()?,
                F_DEST => {
                    let _ = r.string()?;
                }
                F_SIGNATURE => body_sig = r.sig_()?,
                _ => {
                    // Skip unknown field by parsing its variant.
                    let _ = vsig;
                    let vb = vsig.as_bytes();
                    let mut vp = 0;
                    let _ = parse_value(vb, &mut vp, &mut r)?;
                }
            }
        }
    }

    let mut body_raw = vec![0u8; body_len];
    if body_len > 0 {
        read_exact_timeout(stream, &mut body_raw)?;
    }
    let body = if body_sig.is_empty() {
        Vec::new()
    } else {
        parse_body(&body_sig, &body_raw)?
    };

    Ok(Message {
        mtype,
        sender,
        path,
        iface,
        member,
        error_name,
        reply_serial,
        body_sig,
        body,
    })
}

// ---------------------------------------------------------------------------
// Connection
// ---------------------------------------------------------------------------

pub(crate) struct Bus {
    stream: UnixStream,
    serial: u32,
    pub sender: String,
    stash: Vec<Message>,
}

fn write_line(stream: &mut UnixStream, line: &str) -> io::Result<()> {
    stream.write_all(line.as_bytes())?;
    stream.write_all(b"\r\n")
}

fn read_line(stream: &mut UnixStream) -> io::Result<String> {
    let mut out = Vec::new();
    let mut b = [0u8; 1];
    loop {
        read_exact_timeout(stream, &mut b)?;
        if b[0] == b'\n' {
            break;
        }
        out.push(b[0]);
    }
    while out.last() == Some(&b'\r') {
        out.pop();
    }
    String::from_utf8(out).map_err(|e| err(format!("invalid D-Bus auth line: {e}")))
}

fn open_socket() -> io::Result<UnixStream> {
    if let Ok(addr) = std::env::var("DBUS_SESSION_BUS_ADDRESS") {
        for part in addr.split(';') {
            let part = part.trim();
            let rest = match part.strip_prefix("unix:") {
                Some(r) => r,
                None => continue,
            };
            let mut path: Option<&str> = None;
            let mut abs: Option<&str> = None;
            for opt in rest.split(',') {
                if let Some(p) = opt.strip_prefix("path=") {
                    path = Some(p);
                } else if let Some(a) = opt.strip_prefix("abstract=") {
                    abs = Some(a);
                }
            }
            if let Some(p) = path {
                if let Ok(s) = UnixStream::connect(p) {
                    return Ok(s);
                }
            } else if let Some(a) = abs {
                if let Ok(addr) =
                    std::os::unix::net::SocketAddr::from_abstract_name(a.as_bytes())
                {
                    if let Ok(s) = UnixStream::connect_addr(&addr) {
                        return Ok(s);
                    }
                }
            }
        }
    }
    let uid = unsafe { getuid() };
    UnixStream::connect(format!("/run/user/{uid}/bus"))
}

pub(crate) fn connect() -> io::Result<Bus> {
    let mut stream = open_socket().map_err(|e| {
        io::Error::new(
            e.kind(),
            format!(
                "cannot connect to the D-Bus session bus (no desktop session bus found): {e}"
            ),
        )
    })?;
    stream.set_read_timeout(Some(Duration::from_secs(25)))?;
    stream.set_write_timeout(Some(Duration::from_secs(10)))?;

    let uid = unsafe { getuid() };
    let hex: String = format!("{uid}").bytes().map(|b| format!("{b:02x}")).collect();
    stream.write_all(&[0])?;
    write_line(&mut stream, &format!("AUTH EXTERNAL {hex}"))?;
    let line = read_line(&mut stream)?;
    if !line.starts_with("OK") {
        return Err(err(format!("D-Bus auth failed: {line}")));
    }
    write_line(&mut stream, "BEGIN")?;

    let mut bus = Bus {
        stream,
        serial: 0,
        sender: String::new(),
        stash: Vec::new(),
    };
    let (body, _) = bus.call(
        "org.freedesktop.DBus",
        "/org/freedesktop/DBus",
        "org.freedesktop.DBus",
        "Hello",
        "",
        &[],
    )?;
    match body.first() {
        Some(DVal::Str(s)) => bus.sender = s.clone(),
        other => {
            return Err(err(format!("unexpected Hello reply: {other:?}")));
        }
    }
    Ok(bus)
}

impl Bus {
    fn send_call(
        &mut self,
        dest: &str,
        path: &str,
        iface: &str,
        member: &str,
        arg_sig: &str,
        arg_body: &[u8],
    ) -> io::Result<u32> {
        self.serial += 1;
        let serial = self.serial;
        let mut h = Writer::new();
        h.buf.push(b'l');
        h.buf.push(TYPE_METHOD_CALL);
        h.buf.push(0); // flags
        h.buf.push(1); // version
        h.buf.extend_from_slice(&(arg_body.len() as u32).to_le_bytes());
        h.buf.extend_from_slice(&serial.to_le_bytes());
        // Fields array.
        let mut f = Writer::new();
        f.dict_entry(|w| {
            w.u8_(F_PATH);
            w.variant("o", |w| w.path(path));
        });
        f.dict_entry(|w| {
            w.u8_(F_INTERFACE);
            w.variant("s", |w| w.string(iface));
        });
        f.dict_entry(|w| {
            w.u8_(F_MEMBER);
            w.variant("s", |w| w.string(member));
        });
        f.dict_entry(|w| {
            w.u8_(F_DEST);
            w.variant("s", |w| w.string(dest));
        });
        if !arg_sig.is_empty() {
            f.dict_entry(|w| {
                w.u8_(F_SIGNATURE);
                w.variant("g", |w| w.sig_(arg_sig));
            });
        }
        let fields = f.into_bytes();
        h.align(4);
        h.buf.extend_from_slice(&(fields.len() as u32).to_le_bytes());
        h.align(8);
        h.buf.extend_from_slice(&fields);
        h.align(8);
        let mut msg = h.into_bytes();
        msg.extend_from_slice(arg_body);
        self.stream.write_all(&msg)?;
        Ok(serial)
    }

    fn take_stashed(&mut self, serial: u32) -> Option<Message> {
        if let Some(i) = self.stash.iter().position(|m| {
            (m.mtype == TYPE_METHOD_RETURN || m.mtype == TYPE_ERROR) && m.reply_serial == serial
        }) {
            Some(self.stash.remove(i))
        } else {
            None
        }
    }

    /// Calls a method and waits for its reply.
    pub(crate) fn call(
        &mut self,
        dest: &str,
        path: &str,
        iface: &str,
        member: &str,
        arg_sig: &str,
        arg_body: &[u8],
    ) -> io::Result<(Vec<DVal>, String)> {
        let serial = self.send_call(dest, path, iface, member, arg_sig, arg_body)?;
        if let Some(m) = self.take_stashed(serial) {
            return finish_call(m);
        }
        loop {
            let m = read_message(&mut self.stream)?;
            if (m.mtype == TYPE_METHOD_RETURN || m.mtype == TYPE_ERROR)
                && m.reply_serial == serial
            {
                return finish_call(m);
            }
            if self.stash.len() < 64 {
                self.stash.push(m);
            }
        }
    }

    /// Adds a match rule (for signals).
    pub(crate) fn add_match(&mut self, rule: &str) -> io::Result<()> {
        let mut w = Writer::new();
        w.string(rule);
        let body = w.into_bytes();
        let (vals, _) = self.call(
            "org.freedesktop.DBus",
            "/org/freedesktop/DBus",
            "org.freedesktop.DBus",
            "AddMatch",
            "s",
            &body,
        )?;
        let _ = vals;
        Ok(())
    }

    /// Waits for a signal matching `pred` until `deadline`.
    pub(crate) fn wait_signal(
        &mut self,
        deadline: Instant,
        pred: impl Fn(&Message) -> bool,
    ) -> io::Result<Message> {
        if let Some(i) = self.stash.iter().position(|m| m.mtype == TYPE_SIGNAL && pred(m)) {
            return Ok(self.stash.remove(i));
        }
        loop {
            if Instant::now() >= deadline {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "timed out waiting for D-Bus signal",
                ));
            }
            match read_message(&mut self.stream) {
                Ok(m) => {
                    if m.mtype == TYPE_SIGNAL && pred(&m) {
                        return Ok(m);
                    }
                    if self.stash.len() < 64 {
                        self.stash.push(m);
                    }
                }
                Err(e)
                    if e.kind() == io::ErrorKind::TimedOut
                        || e.kind() == io::ErrorKind::WouldBlock =>
                {
                    return Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        "timed out waiting for D-Bus signal",
                    ));
                }
                Err(e) => return Err(e),
            }
        }
    }
}

fn finish_call(m: Message) -> io::Result<(Vec<DVal>, String)> {
    if m.mtype == TYPE_ERROR {
        let detail = m
            .body
            .first()
            .and_then(|v| v.as_str())
            .unwrap_or("no details");
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("D-Bus error {}: {}", m.error_name, detail),
        ));
    }
    Ok((m.body, m.body_sig))
}
