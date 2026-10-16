//! Linux `uinput` backend: kernel-level virtual input device.
//!
//! Used for keyboard/mouse input on pure-Wayland sessions where no X server
//! (not even XWayland) is available. Works on any compositor because events
//! are injected at the kernel `evdev` layer (like `ydotool`).
//!
//! Requires read/write access to `/dev/uinput` (usually the `input` group
//! plus a udev rule, or root). All raw numbers below come from
//! `linux/input-event-codes.h` and `linux/uinput.h`.

use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::os::raw::{c_int, c_ulong, c_void};
use std::os::unix::io::AsRawFd;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::keyboard::Key;

// ---------------------------------------------------------------------------
// evdev constants
// ---------------------------------------------------------------------------

const EV_SYN: u16 = 0x00;
const EV_KEY: u16 = 0x01;
const EV_REL: u16 = 0x02;
const SYN_REPORT: u16 = 0;

const REL_X: u16 = 0x00;
const REL_Y: u16 = 0x01;
const REL_HWHEEL: u16 = 0x06;
const REL_WHEEL: u16 = 0x08;

pub(crate) const BTN_LEFT: u16 = 0x110;
pub(crate) const BTN_RIGHT: u16 = 0x111;
pub(crate) const BTN_MIDDLE: u16 = 0x112;
pub(crate) const BTN_X1: u16 = 0x113; // BTN_SIDE
pub(crate) const BTN_X2: u16 = 0x114; // BTN_EXTRA

const BUS_USB: u16 = 0x03;

// uinput ioctls (from linux/uinput.h ioctl encoding).
const UI_SET_EVBIT: c_ulong = 0x4004_5564;
const UI_SET_KEYBIT: c_ulong = 0x4004_5565;
const UI_SET_RELBIT: c_ulong = 0x4004_5566;
const UI_DEV_SETUP: c_ulong = 0x405C_5503;
const UI_DEV_CREATE: c_ulong = 0x5501;
#[allow(dead_code)]
const UI_DEV_DESTROY: c_ulong = 0x5502;

extern "C" {
    fn ioctl(fd: c_int, req: c_ulong, ...) -> c_int;
}

#[repr(C)]
struct UinputSetup {
    bustype: u16,
    vendor: u16,
    product: u16,
    version: u16,
    name: [u8; 80],
    ff_effects_max: u32,
}

// ---------------------------------------------------------------------------
// Device lifetime (created once per process)
// ---------------------------------------------------------------------------

struct Device {
    file: File,
}

fn open_uinput() -> io::Result<File> {
    match OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/uinput")
    {
        Ok(f) => Ok(f),
        Err(e) if e.kind() == io::ErrorKind::NotFound => OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/input/uinput"),
        Err(e) => Err(e),
    }
}

impl Device {
    fn create() -> io::Result<Self> {
        let file = open_uinput()?;
        let fd = file.as_raw_fd();

        let ok = |r: c_int| r == 0;
        unsafe {
            if !ok(ioctl(fd, UI_SET_EVBIT, EV_KEY as c_int)) {
                return Err(io::Error::last_os_error());
            }
            if !ok(ioctl(fd, UI_SET_EVBIT, EV_REL as c_int)) {
                return Err(io::Error::last_os_error());
            }
            // Enable every key/button code (1..=767 covers KEY_MAX and all
            // BTN_*). Enabling unused codes is harmless and avoids keeping a
            // second list in sync with the mapping tables below.
            for code in 1u32..=767 {
                if !ok(ioctl(fd, UI_SET_KEYBIT, code as c_int)) {
                    return Err(io::Error::last_os_error());
                }
            }
            for rel in [REL_X, REL_Y, REL_WHEEL, REL_HWHEEL] {
                if !ok(ioctl(fd, UI_SET_RELBIT, rel as c_int)) {
                    return Err(io::Error::last_os_error());
                }
            }

            let mut name = [0u8; 80];
            let tag = b"rsautogui";
            name[..tag.len()].copy_from_slice(tag);
            let setup = UinputSetup {
                bustype: BUS_USB,
                vendor: 0x1,
                product: 0x1,
                version: 1,
                name,
                ff_effects_max: 0,
            };
            if !ok(ioctl(
                fd,
                UI_DEV_SETUP,
                &setup as *const UinputSetup as *const c_void,
            )) {
                return Err(io::Error::last_os_error());
            }
            if !ok(ioctl(fd, UI_DEV_CREATE, 0)) {
                return Err(io::Error::last_os_error());
            }
        }

        // The device node appears asynchronously; give it a moment so the
        // first events are not dropped. One-time cost per process.
        std::thread::sleep(std::time::Duration::from_millis(300));
        Ok(Self { file })
    }
}

fn device() -> &'static Mutex<Device> {
    static DEV: OnceLock<Mutex<Device>> = OnceLock::new();
    DEV.get_or_init(|| match Device::create() {
        Ok(d) => Mutex::new(d),
        Err(e) if e.kind() == io::ErrorKind::PermissionDenied => panic!(
            "rsautogui: cannot open /dev/uinput (permission denied). Grant access, e.g. \
             `sudo usermod -aG input $USER` (then re-login) with a udev rule \
             `KERNEL==\"uinput\", GROUP=\"input\", MODE=\"0660\"`, or force the X11 backend \
             under XWayland with RSAUTOGUI_BACKEND=x11. Original error: {e}"
        ),
        Err(e) => panic!(
            "rsautogui: cannot set up uinput device (/dev/uinput missing? load it with \
             `sudo modprobe uinput`): {e}"
        ),
    })
}

// ---------------------------------------------------------------------------
// Event emission
// ---------------------------------------------------------------------------

fn timestamp() -> (i64, i64) {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| (d.as_secs() as i64, d.subsec_micros() as i64))
        .unwrap_or((0, 0))
}

fn emit(file: &mut File, type_: u16, code: u16, value: i32) -> io::Result<()> {
    // struct input_event: timeval (2 x 8 bytes) + type + code + value = 24.
    let (sec, usec) = timestamp();
    let mut b = [0u8; 24];
    b[0..8].copy_from_slice(&sec.to_le_bytes());
    b[8..16].copy_from_slice(&usec.to_le_bytes());
    b[16..18].copy_from_slice(&type_.to_le_bytes());
    b[18..20].copy_from_slice(&code.to_le_bytes());
    b[20..24].copy_from_slice(&value.to_le_bytes());
    file.write_all(&b)
}

fn syn(file: &mut File) -> io::Result<()> {
    emit(file, EV_SYN, SYN_REPORT, 0)
}

fn with_device(f: impl FnOnce(&mut File) -> io::Result<()>) {
    let dev = device();
    let mut g = dev.lock().unwrap_or_else(|e| e.into_inner());
    f(&mut g.file).expect("rsautogui: failed writing to /dev/uinput");
}

/// Press (down=true) or release a key/button code.
pub(crate) fn key_event(code: u16, down: bool) {
    with_device(|f| {
        emit(f, EV_KEY, code, if down { 1 } else { 0 })?;
        syn(f)
    });
}

/// Relative pointer motion.
pub(crate) fn rel_move(dx: i32, dy: i32) {
    if dx == 0 && dy == 0 {
        return;
    }
    with_device(|f| {
        if dx != 0 {
            emit(f, EV_REL, REL_X, dx)?;
        }
        if dy != 0 {
            emit(f, EV_REL, REL_Y, dy)?;
        }
        syn(f)
    });
}

/// Wheel scroll. Positive = up (vertical) / right (horizontal).
pub(crate) fn scroll(vertical: bool, lines: i32) {
    if lines == 0 {
        return;
    }
    with_device(|f| {
        emit(f, EV_REL, if vertical { REL_WHEEL } else { REL_HWHEEL }, lines)?;
        syn(f)
    });
}

// ---------------------------------------------------------------------------
// Key -> evdev mapping
// ---------------------------------------------------------------------------

/// Maps a [`Key`] to a Linux evdev `KEY_*` code, or `None` if unsupported.
#[allow(clippy::just_underscores_and_digits)]
pub(crate) fn evdev_key_opt(key: Key) -> Option<u16> {
    use Key::*;
    Some(match key {
        Escape => 1,
        _1 => 2,
        _2 => 3,
        _3 => 4,
        _4 => 5,
        _5 => 6,
        _6 => 7,
        _7 => 8,
        _8 => 9,
        _9 => 10,
        _0 => 11,
        Minus => 12,
        Plus => 13, // '=' key
        Backspace => 14,
        Tab => 15,
        Q => 16,
        W => 17,
        E => 18,
        R => 19,
        T => 20,
        Y => 21,
        U => 22,
        I => 23,
        O => 24,
        P => 25,
        Oem4 => 26, // '['
        Oem6 => 27, // ']'
        Enter => 28,
        Control | LeftControl => 29,
        A => 30,
        S => 31,
        D => 32,
        F => 33,
        G => 34,
        H => 35,
        J => 36,
        K => 37,
        L => 38,
        Oem1 => 39, // ';'
        Oem7 => 40, // '\''
        Oem3 => 41, // '`'
        Shift | LeftShift => 42,
        Oem5 => 43, // '\'
        Z => 44,
        X => 45,
        C => 46,
        V => 47,
        B => 48,
        N => 49,
        M => 50,
        Comma => 51,
        Period => 52,
        Oem2 => 53, // '/'
        RightShift => 54,
        Multiply => 55, // keypad '*'
        Alt | LeftMenu => 56,
        Space => 57,
        CapsLock => 58,
        F1 => 59,
        F2 => 60,
        F3 => 61,
        F4 => 62,
        F5 => 63,
        F6 => 64,
        F7 => 65,
        F8 => 66,
        F9 => 67,
        F10 => 68,
        Numlock => 69,
        Scroll => 70,
        Numpad7 => 71,
        Numpad8 => 72,
        Numpad9 => 73,
        Subtract => 74, // keypad '-'
        Numpad4 => 75,
        Numpad5 => 76,
        Numpad6 => 77,
        Add => 78, // keypad '+'
        Numpad1 => 79,
        Numpad2 => 80,
        Numpad3 => 81,
        Numpad0 => 82,
        Decimal | Separator => 83, // keypad '.'
        Oem102 => 86,              // KEY_102ND (<> on 102-key boards)
        F11 => 87,
        F12 => 88,
        RightControl => 97,
        Divide => 98, // keypad '/'
        PrintScreen | Print => 99, // KEY_SYSRQ
        RightMenu => 100,          // right Alt
        Home => 102,
        UpArrow => 103,
        PageUp => 104,
        LeftArrow => 105,
        RightArrow => 106,
        End => 107,
        DownArrow => 108,
        PageDown => 109,
        Insert => 110,
        Delete => 111,
        VolumeMute => 113,
        VolumeDown => 114,
        VolumeUp => 115,
        Pause => 119,
        LeftWin => 125,  // LEFTMETA
        RightWin => 126, // RIGHTMETA
        BrowserStop => 128,
        Sleep => 142,
        StartMail => 155,
        BrowserFavorites => 156,
        BrowserBack => 158,
        BrowserForward => 159,
        NextTrack => 163,
        MediaPlayPause | Play => 164,
        PrevTrack => 165,
        MediaStop => 166,
        BrowserHome => 172,
        BrowserRefresh => 173,
        F13 => 183,
        F14 => 184,
        F15 => 185,
        F16 => 186,
        F17 => 187,
        F18 => 188,
        F19 => 189,
        F20 => 190,
        F21 => 191,
        F22 => 192,
        F23 => 193,
        F24 => 194,
        BrowserSearch => 217,
        Kana => 93,      // KEY_KATAKANAHIRAGANA
        Convert => 92,   // KEY_HENKAN
        NonConvert => 94, // KEY_MUHENKAN
        Kanji => 94,     // closest: MUHENKAN-adjacent Kanji handling
        Zoom => 372,     // KEY_ZOOM
        MouseLeft | MouseRight | MouseMiddle | MouseX1 | MouseX2 => {
            panic!("rsautogui: mouse buttons must be used via mouse::click, not keyboard")
        }
        _ => return None,
    })
}

/// Maps a [`Key`] to a Linux evdev `KEY_*` code.
///
/// Panics for keys with no evdev equivalent.
pub(crate) fn evdev_key(key: Key) -> u16 {
    match evdev_key_opt(key) {
        Some(code) => code,
        None => panic!(
            "rsautogui: key {:?} has no evdev equivalent for the uinput backend",
            key
        ),
    }
}

/// Maps a character to `(evdev code, needs_shift)` assuming a US layout.
///
/// Only ASCII is supported: arbitrary Unicode cannot be expressed through
/// evdev key codes without a compositor-side input method.
pub(crate) fn evdev_char(c: char) -> (u16, bool) {
    let (code, shift): (u16, bool) = match c {
        'a' => (30, false),
        'b' => (48, false),
        'c' => (46, false),
        'd' => (32, false),
        'e' => (18, false),
        'f' => (33, false),
        'g' => (34, false),
        'h' => (35, false),
        'i' => (23, false),
        'j' => (36, false),
        'k' => (37, false),
        'l' => (38, false),
        'm' => (50, false),
        'n' => (49, false),
        'o' => (24, false),
        'p' => (25, false),
        'q' => (16, false),
        'r' => (19, false),
        's' => (31, false),
        't' => (20, false),
        'u' => (22, false),
        'v' => (47, false),
        'w' => (17, false),
        'x' => (45, false),
        'y' => (21, false),
        'z' => (44, false),
        '1' => (2, false),
        '2' => (3, false),
        '3' => (4, false),
        '4' => (5, false),
        '5' => (6, false),
        '6' => (7, false),
        '7' => (8, false),
        '8' => (9, false),
        '9' => (10, false),
        '0' => (11, false),
        '-' => (12, false),
        '=' => (13, false),
        '[' => (26, false),
        ']' => (27, false),
        '\\' => (43, false),
        ';' => (39, false),
        '\'' => (40, false),
        '`' => (41, false),
        ',' => (51, false),
        '.' => (52, false),
        '/' => (53, false),
        ' ' => (57, false),
        'A' => (30, true),
        'B' => (48, true),
        'C' => (46, true),
        'D' => (32, true),
        'E' => (18, true),
        'F' => (33, true),
        'G' => (34, true),
        'H' => (35, true),
        'I' => (23, true),
        'J' => (36, true),
        'K' => (37, true),
        'L' => (38, true),
        'M' => (50, true),
        'N' => (49, true),
        'O' => (24, true),
        'P' => (25, true),
        'Q' => (16, true),
        'R' => (19, true),
        'S' => (31, true),
        'T' => (20, true),
        'U' => (22, true),
        'V' => (47, true),
        'W' => (17, true),
        'X' => (45, true),
        'Y' => (21, true),
        'Z' => (44, true),
        '!' => (2, true),
        '@' => (3, true),
        '#' => (4, true),
        '$' => (5, true),
        '%' => (6, true),
        '^' => (7, true),
        '&' => (8, true),
        '*' => (9, true),
        '(' => (10, true),
        ')' => (11, true),
        '_' => (12, true),
        '+' => (13, true),
        '{' => (26, true),
        '}' => (27, true),
        '|' => (43, true),
        ':' => (39, true),
        '"' => (40, true),
        '~' => (41, true),
        '<' => (51, true),
        '>' => (52, true),
        '?' => (53, true),
        '\n' | '\r' => (28, false),
        '\t' => (15, false),
        _ => panic!(
            "rsautogui: cannot type {:?} via uinput (US-ASCII only on pure Wayland; \
             use an XWayland session for full Unicode typing)",
            c
        ),
    };
    (code, shift)
}
