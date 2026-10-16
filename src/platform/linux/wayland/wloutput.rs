//! Minimal Wayland client: connects to the compositor and reads the first
//! `wl_output`'s current mode and scale.
//!
//! Only used on Wayland sessions where X (XWayland) is unavailable, to learn
//! the screen size (and buffer scale for screenshots). Hand-rolled wire
//! protocol over the Wayland socket — no dependencies.

use std::io::{self, Read, Write};
use std::os::unix::net::UnixStream;
use std::time::Duration;

extern "C" {
    fn getuid() -> u32;
}

fn err(msg: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::Other, msg.into())
}

fn socket() -> io::Result<UnixStream> {
    let path = if let Ok(d) = std::env::var("WAYLAND_DISPLAY") {
        if d.starts_with('/') {
            d
        } else {
            format!("{}/{}", runtime_dir(), d)
        }
    } else {
        format!("{}/wayland-0", runtime_dir())
    };
    let s = UnixStream::connect(&path).map_err(|e| {
        io::Error::new(
            e.kind(),
            format!("cannot connect to Wayland compositor at {path}: {e}"),
        )
    })?;
    s.set_read_timeout(Some(Duration::from_secs(5)))?;
    s.set_write_timeout(Some(Duration::from_secs(5)))?;
    Ok(s)
}

fn runtime_dir() -> String {
    if let Ok(d) = std::env::var("XDG_RUNTIME_DIR") {
        if !d.is_empty() {
            return d;
        }
    }
    let uid = unsafe { getuid() };
    format!("/run/user/{uid}")
}

fn put_string(buf: &mut Vec<u8>, s: &str) {
    buf.extend_from_slice(&((s.len() + 1) as u32).to_le_bytes());
    buf.extend_from_slice(s.as_bytes());
    buf.push(0);
    while buf.len() % 4 != 0 {
        buf.push(0);
    }
}

fn send(s: &mut UnixStream, obj: u32, opcode: u16, payload: &[u8]) -> io::Result<()> {
    let size = (8 + payload.len()) as u32;
    let mut msg = Vec::with_capacity(size as usize);
    msg.extend_from_slice(&obj.to_le_bytes());
    msg.extend_from_slice(&(((size << 16) | opcode as u32).to_le_bytes()));
    msg.extend_from_slice(payload);
    s.write_all(&msg)
}

struct Cursor<'a> {
    b: &'a [u8],
    off: usize,
}

impl<'a> Cursor<'a> {
    fn need(&self, n: usize) -> io::Result<()> {
        if self.off + n > self.b.len() {
            return Err(err("truncated Wayland message"));
        }
        Ok(())
    }
    fn u32_(&mut self) -> io::Result<u32> {
        self.need(4)?;
        let v = u32::from_le_bytes([
            self.b[self.off],
            self.b[self.off + 1],
            self.b[self.off + 2],
            self.b[self.off + 3],
        ]);
        self.off += 4;
        Ok(v)
    }
    fn i32_(&mut self) -> io::Result<i32> {
        Ok(self.u32_()? as i32)
    }
    fn string(&mut self) -> io::Result<String> {
        let len = self.u32_()? as usize;
        if len == 0 {
            return Err(err("bad Wayland string"));
        }
        self.need(len)?;
        let s = std::str::from_utf8(&self.b[self.off..self.off + len - 1])
            .map_err(|e| err(format!("bad Wayland string: {e}")))?
            .to_owned();
        self.off += len;
        while self.off % 4 != 0 {
            self.off += 1;
        }
        if self.off > self.b.len() {
            return Err(err("truncated Wayland message"));
        }
        Ok(s)
    }
}

struct Event {
    obj: u32,
    opcode: u16,
    payload: Vec<u8>,
}

fn read_event(s: &mut UnixStream) -> io::Result<Event> {
    let mut hdr = [0u8; 8];
    s.read_exact(&mut hdr)?;
    let obj = u32::from_le_bytes(hdr[0..4].try_into().unwrap());
    let size_op = u32::from_le_bytes(hdr[4..8].try_into().unwrap());
    let opcode = (size_op & 0xFFFF) as u16;
    let size = (size_op >> 16) as usize;
    if !(8..=64 * 1024).contains(&size) {
        return Err(err("bad Wayland message size"));
    }
    let mut payload = vec![0u8; size - 8];
    if !payload.is_empty() {
        s.read_exact(&mut payload)?;
    }
    Ok(Event { obj, opcode, payload })
}

/// Returns `(width, height, scale)` of the first output.
pub(crate) fn output_info() -> io::Result<(u16, u16, i32)> {
    let mut s = socket()?;

    // wl_display.get_registry(new id 2) + sync (callback 3).
    let mut p = Vec::new();
    p.extend_from_slice(&2u32.to_le_bytes());
    send(&mut s, 1, 1, &p)?;
    let mut p = Vec::new();
    p.extend_from_slice(&3u32.to_le_bytes());
    send(&mut s, 1, 0, &p)?;

    let mut outputs: Vec<(u32, u32)> = Vec::new(); // (name, version)
    loop {
        let e = read_event(&mut s)?;
        if e.obj == 1 && e.opcode == 0 {
            let mut c = Cursor { b: &e.payload, off: 0 };
            let _ = c.u32_()?;
            let _ = c.u32_()?;
            let msg = c.string().unwrap_or_default();
            return Err(err(format!("Wayland error: {msg}")));
        }
        if e.obj == 2 && e.opcode == 0 {
            let mut c = Cursor { b: &e.payload, off: 0 };
            let name = c.u32_()?;
            let iface = c.string()?;
            let ver = c.u32_()?;
            if iface == "wl_output" {
                outputs.push((name, ver));
            }
        } else if e.obj == 3 && e.opcode == 0 {
            break; // first roundtrip done
        }
    }

    let (name, ver) = outputs
        .first()
        .copied()
        .ok_or_else(|| err("compositor advertised no wl_output"))?;
    let bind_ver = ver.min(2);
    let mut p = Vec::new();
    p.extend_from_slice(&name.to_le_bytes());
    put_string(&mut p, "wl_output");
    p.extend_from_slice(&bind_ver.to_le_bytes());
    p.extend_from_slice(&4u32.to_le_bytes());
    send(&mut s, 2, 0, &p)?; // wl_registry.bind

    let mut p = Vec::new();
    p.extend_from_slice(&5u32.to_le_bytes());
    send(&mut s, 1, 0, &p)?; // sync (callback 5)

    let (mut w, mut h) = (0i32, 0i32);
    let mut scale = 1i32;
    loop {
        let e = read_event(&mut s)?;
        if e.obj == 1 && e.opcode == 0 {
            let mut c = Cursor { b: &e.payload, off: 0 };
            let _ = c.u32_()?;
            let _ = c.u32_()?;
            let msg = c.string().unwrap_or_default();
            return Err(err(format!("Wayland error: {msg}")));
        }
        if e.obj == 4 {
            let mut c = Cursor { b: &e.payload, off: 0 };
            match e.opcode {
                0 => {
                    // geometry: x y phys_w phys_h subpixel make model transform
                    for _ in 0..5 {
                        c.i32_()?;
                    }
                    c.string()?;
                    c.string()?;
                    c.i32_()?;
                }
                1 => {
                    // mode: flags w h refresh
                    let flags = c.u32_()?;
                    let mw = c.i32_()?;
                    let mh = c.i32_()?;
                    let _ = c.i32_()?;
                    if flags & 1 != 0 {
                        w = mw;
                        h = mh;
                    }
                }
                3 => {
                    scale = c.i32_()?;
                }
                _ => {} // done(2) and others need no parsing
            }
        } else if e.obj == 5 && e.opcode == 0 {
            break; // second roundtrip done: all output events received
        }
    }

    if w <= 0 || h <= 0 {
        return Err(err("wl_output reported no current mode"));
    }
    Ok((w as u16, h as u16, scale.max(1)))
}
