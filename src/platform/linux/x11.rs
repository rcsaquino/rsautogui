//! Linux backend: X11 (`libX11`) + XTest (`libXtst`).
//!
//! Requires an X server (`$DISPLAY`). Wayland-native sessions are not
//! supported — run under XWayland instead.
#![allow(clippy::all)]

use std::os::raw::{c_char, c_int, c_uchar, c_uint, c_ulong, c_void};
use std::ptr::null;

use crate::image::Image;
use crate::keyboard::Key;
use crate::mouse::Button;

// ---------------------------------------------------------------------------
// XImage layout (prefix must match Xlib.h; we only read up to blue_mask).
// ---------------------------------------------------------------------------

#[repr(C)]
struct XImage {
    width: c_int,
    height: c_int,
    xoffset: c_int,
    format: c_int,
    data: *mut c_char,
    byte_order: c_int,
    bitmap_unit: c_int,
    bitmap_bit_order: c_int,
    bitmap_pad: c_int,
    depth: c_int,
    bytes_per_line: c_int,
    bits_per_pixel: c_int,
    red_mask: c_ulong,
    green_mask: c_ulong,
    blue_mask: c_ulong,
    obdata: *mut c_void,
    f: *mut c_void,
}

const ZPIXMAP: c_int = 2;
const LSB_FIRST: c_int = 0;
#[allow(dead_code)]
const MSB_FIRST: c_int = 1;

#[link(name = "X11")]
extern "C" {
    fn XOpenDisplay(display_name: *const c_char) -> *mut c_void;
    fn XCloseDisplay(display: *mut c_void) -> c_int;
    fn XDefaultScreen(display: *mut c_void) -> c_int;
    fn XDisplayWidth(display: *mut c_void, screen_number: c_int) -> c_int;
    fn XDisplayHeight(display: *mut c_void, screen_number: c_int) -> c_int;
    fn XDefaultRootWindow(display: *mut c_void) -> c_ulong;
    fn XQueryPointer(
        display: *mut c_void,
        w: c_ulong,
        root_return: *mut c_ulong,
        child_return: *mut c_ulong,
        root_x_return: *mut c_int,
        root_y_return: *mut c_int,
        win_x_return: *mut c_int,
        win_y_return: *mut c_int,
        mask_return: *mut c_uint,
    ) -> c_int;
    fn XWarpPointer(
        display: *mut c_void,
        src_w: c_ulong,
        dest_w: c_ulong,
        src_x: c_int,
        src_y: c_int,
        src_width: c_uint,
        src_height: c_uint,
        dest_x: c_int,
        dest_y: c_int,
    ) -> c_int;
    fn XFlush(display: *mut c_void) -> c_int;
    fn XKeysymToKeycode(display: *mut c_void, keysym: c_ulong) -> c_uchar;
    fn XGetImage(
        display: *mut c_void,
        d: c_ulong,
        x: c_int,
        y: c_int,
        width: c_uint,
        height: c_uint,
        plane_mask: c_ulong,
        format: c_int,
    ) -> *mut XImage;
    fn XDestroyImage(image: *mut XImage) -> c_int;
    fn XQueryKeymap(display: *mut c_void, keys: *mut c_char) -> c_int;
}

#[link(name = "Xtst")]
extern "C" {
    fn XTestFakeButtonEvent(
        display: *mut c_void,
        button: c_uint,
        is_press: c_int,
        delay: c_ulong,
    ) -> c_int;
    fn XTestFakeMotionEvent(
        display: *mut c_void,
        screen_number: c_int,
        x: c_int,
        y: c_int,
        delay: c_ulong,
    ) -> c_int;
    fn XTestFakeRelativeMotionEvent(
        display: *mut c_void,
        x: c_int,
        y: c_int,
        delay: c_ulong,
    ) -> c_int;
    fn XTestFakeKeyEvent(
        display: *mut c_void,
        keycode: c_uint,
        is_press: c_int,
        delay: c_ulong,
    ) -> c_int;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn open_display() -> *mut c_void {
    unsafe {
        let dpy = XOpenDisplay(null());
        if dpy.is_null() {
            panic!(
                "rsautogui: cannot open X display. Linux requires X11 (or XWayland); \
                 is $DISPLAY set? Wayland-native sessions are not supported."
            );
        }
        dpy
    }
}

unsafe fn close_display(dpy: *mut c_void) {
    XCloseDisplay(dpy);
}

fn button_number(button: Button) -> c_uint {
    match button {
        Button::Left => 1,
        Button::Middle => 2,
        Button::Right => 3,
        // X11: 8 = back / X1, 9 = forward / X2.
        Button::X1 => 8,
        Button::X2 => 9,
    }
}

fn keysym_to_keycode(dpy: *mut c_void, keysym: c_ulong, what: &str) -> c_uint {
    unsafe {
        let kc = XKeysymToKeycode(dpy, keysym);
        if kc == 0 {
            panic!(
                "rsautogui: no X keycode for {} (keysym 0x{:X}); \
                 this key may not exist on the current layout",
                what, keysym
            );
        }
        kc as c_uint
    }
}

fn fake_key(dpy: *mut c_void, keycode: c_uint, press: bool) {
    unsafe {
        XTestFakeKeyEvent(dpy, keycode, if press { 1 } else { 0 }, 0);
        XFlush(dpy);
    }
}

/// Maps a [`Key`] to an X11 keysym.
///
/// Panics for keys with no X11 equivalent.
fn key_to_keysym(key: Key) -> c_ulong {
    use Key::*;
    let ks: u64 = match key {
        A => 0x061,
        B => 0x062,
        C => 0x063,
        D => 0x064,
        E => 0x065,
        F => 0x066,
        G => 0x067,
        H => 0x068,
        I => 0x069,
        J => 0x06A,
        K => 0x06B,
        L => 0x06C,
        M => 0x06D,
        N => 0x06E,
        O => 0x06F,
        P => 0x070,
        Q => 0x071,
        R => 0x072,
        S => 0x073,
        T => 0x074,
        U => 0x075,
        V => 0x076,
        W => 0x077,
        X => 0x078,
        Y => 0x079,
        Z => 0x07A,
        _0 => 0x030,
        _1 => 0x031,
        _2 => 0x032,
        _3 => 0x033,
        _4 => 0x034,
        _5 => 0x035,
        _6 => 0x036,
        _7 => 0x037,
        _8 => 0x038,
        _9 => 0x039,
        Space => 0x020,
        Backspace => 0xFF08,
        Tab => 0xFF09,
        Clear => 0xFF0B,
        Enter => 0xFF0D,
        Pause => 0xFF13,
        Escape => 0xFF1B,
        PageUp => 0xFF55,
        PageDown => 0xFF56,
        End => 0xFF57,
        Home => 0xFF50,
        LeftArrow => 0xFF51,
        UpArrow => 0xFF52,
        RightArrow => 0xFF53,
        DownArrow => 0xFF54,
        Select => 0xFF60,
        Print => 0xFF61,
        Execute => 0xFF62,
        PrintScreen => 0xFF61,
        Insert => 0xFF63,
        Delete => 0xFFFF,
        Help => 0xFF6A,
        LeftWin => 0xFFEB,  // Super_L
        RightWin => 0xFFEC, // Super_R
        Apps => 0xFF67,              // Menu
        Sleep => 0x1008_FF2F,        // XF86Sleep
        Numpad0 => 0xFFB0,
        Numpad1 => 0xFFB1,
        Numpad2 => 0xFFB2,
        Numpad3 => 0xFFB3,
        Numpad4 => 0xFFB4,
        Numpad5 => 0xFFB5,
        Numpad6 => 0xFFB6,
        Numpad7 => 0xFFB7,
        Numpad8 => 0xFFB8,
        Numpad9 => 0xFFB9,
        Multiply => 0xFFAA,
        Add => 0xFFAB,
        Separator => 0xFFAC,
        Subtract => 0xFFAD,
        Decimal => 0xFFAE,
        Divide => 0xFFAF,
        F1 => 0xFFBE,
        F2 => 0xFFBF,
        F3 => 0xFFC0,
        F4 => 0xFFC1,
        F5 => 0xFFC2,
        F6 => 0xFFC3,
        F7 => 0xFFC4,
        F8 => 0xFFC5,
        F9 => 0xFFC6,
        F10 => 0xFFC7,
        F11 => 0xFFC8,
        F12 => 0xFFC9,
        F13 => 0xFFCA,
        F14 => 0xFFCB,
        F15 => 0xFFCC,
        F16 => 0xFFCD,
        F17 => 0xFFCE,
        F18 => 0xFFCF,
        F19 => 0xFFD0,
        F20 => 0xFFD1,
        F21 => 0xFFD2,
        F22 => 0xFFD3,
        F23 => 0xFFD4,
        F24 => 0xFFD5,
        Numlock => 0xFF7F,
        Scroll => 0xFF14,
        Shift | LeftShift => 0xFFE1,
        RightShift => 0xFFE2,
        Control | LeftControl => 0xFFE3,
        RightControl => 0xFFE4,
        CapsLock => 0xFFE5,
        Alt | LeftMenu => 0xFFE9,
        RightMenu => 0xFFEA,
        BrowserBack => 0x1008_FF26,
        BrowserForward => 0x1008_FF27,
        BrowserRefresh => 0x1008_FF73,
        BrowserStop => 0x1008_FF28,
        BrowserSearch => 0x1008_FF1B,
        BrowserFavorites => 0x1008_FF30,
        BrowserHome => 0x1008_FF18,
        VolumeMute => 0x1008_FF12,
        VolumeDown => 0x1008_FF11,
        VolumeUp => 0x1008_FF13,
        NextTrack => 0x1008_FF17,
        PrevTrack => 0x1008_FF16,
        MediaStop => 0x1008_FF15,
        MediaPlayPause => 0x1008_FF14,
        StartMail => 0x1008_FF19,
        SelectMedia => 0x1008_FF32,
        StartApp1 => 0x1008_FF2A, // XF86LaunchA-ish fallback
        StartApp2 => 0x1008_FF2B,
        Oem1 => 0x03B, // ';'
        Plus => 0x03D, // '='
        Comma => 0x02C,
        Minus => 0x02D,
        Period => 0x02E,
        Oem2 => 0x02F, // '/'
        Oem3 => 0x060, // '`'
        Oem4 => 0x05B, // '['
        Oem5 => 0x05C, // '\'
        Oem6 => 0x05D, // ']'
        Oem7 => 0x027, // '\''
        Oem8 => 0x060,
        Oem102 => 0x03C, // '<'
        Kana => 0xFF26,
        Kanji => 0xFF21,
        Junja => 0xFF27,
        Final => 0xFF28,
        Convert => 0xFF23,
        NonConvert => 0xFF22,
        Accept => 0xFF23,
        ModeChange => 0xFF7E,
        ImeOn => 0xFF27,
        ImeOff => 0xFF22,
        ImeProcess => 0xFF23,
        Attn => 0xFF6B,
        CrSel => 0xFF6C,
        ExSel => 0xFF6D,
        EraseEof => 0xFF6E,
        Play => 0x1008_FF14,
        Zoom => 0x1008_FF33,
        Pa1 => 0xFF6B,
        OemClear => 0xFF0B,
        Cancel => 0xFF69,
        // Mouse buttons used as keys: handled by delegation, never reach here.
        MouseLeft | MouseRight | MouseMiddle | MouseX1 | MouseX2 => {
            panic!("rsautogui: mouse buttons must be used via mouse::click, not keyboard")
        }
    };
    ks as c_ulong
}

/// For a character, returns `(base_keysym, needs_shift)` assuming a US layout.
fn char_keysym_and_shift(c: char) -> (c_ulong, bool) {
    let (base, shift) = match c {
        'a'..='z' | '0'..='9' | ' ' | '`' | '-' | '=' | '[' | ']' | '\\' | ';' | '\'' | ','
        | '.' | '/' => (c as u64, false),
        'A'..='Z' => (c.to_ascii_lowercase() as u64, true),
        '!' => ('1' as u64, true),
        '@' => ('2' as u64, true),
        '#' => ('3' as u64, true),
        '$' => ('4' as u64, true),
        '%' => ('5' as u64, true),
        '^' => ('6' as u64, true),
        '&' => ('7' as u64, true),
        '*' => ('8' as u64, true),
        '(' => ('9' as u64, true),
        ')' => ('0' as u64, true),
        '_' => ('-' as u64, true),
        '+' => ('=' as u64, true),
        '{' => ('[' as u64, true),
        '}' => (']' as u64, true),
        '|' => ('\\' as u64, true),
        ':' => (';' as u64, true),
        '"' => ('\'' as u64, true),
        '<' => (',' as u64, true),
        '>' => ('.' as u64, true),
        '?' => ('/' as u64, true),
        '~' => ('`' as u64, true),
        '\n' | '\r' => (0xFF0D, false),
        '\t' => (0xFF09, false),
        _ => {
            let cp = c as u32 as u64;
            if cp < 0x100 {
                (cp, false)
            } else {
                // X11 Unicode keysym: U+01000000 | codepoint.
                (0x0100_0000 | cp, false)
            }
        }
    };
    (base as c_ulong, shift)
}

fn shift_keycode(dpy: *mut c_void) -> c_uint {
    keysym_to_keycode(dpy, 0xFFE1, "Shift")
}

// ---------------------------------------------------------------------------
// Mouse
// ---------------------------------------------------------------------------

pub(crate) fn mouse_position() -> (u16, u16) {
    unsafe {
        let dpy = open_display();
        let root = XDefaultRootWindow(dpy);
        let (mut root_ret, mut child_ret) = (0, 0);
        let (mut rx, mut ry, mut wx, mut wy) = (0, 0, 0, 0);
        let mut mask = 0;
        let ok = XQueryPointer(
            dpy, root, &mut root_ret, &mut child_ret, &mut rx, &mut ry, &mut wx, &mut wy,
            &mut mask,
        );
        close_display(dpy);
        if ok == 0 {
            panic!("rsautogui: unable to locate mouse position");
        }
        (rx.max(0) as u16, ry.max(0) as u16)
    }
}

pub(crate) fn mouse_set_position(x: u16, y: u16) {
    unsafe {
        let dpy = open_display();
        let screen = XDefaultScreen(dpy);
        let root = XDefaultRootWindow(dpy);
        XWarpPointer(dpy, 0, root, 0, 0, 0, 0, x as c_int, y as c_int);
        // Also emit an XTest motion so toolkits that listen to XInput see it.
        XTestFakeMotionEvent(dpy, screen, x as c_int, y as c_int, 0);
        XFlush(dpy);
        close_display(dpy);
    }
}

pub(crate) fn mouse_move_relative(dx: i32, dy: i32) {
    unsafe {
        let dpy = open_display();
        XTestFakeRelativeMotionEvent(dpy, dx as c_int, dy as c_int, 0);
        XFlush(dpy);
        close_display(dpy);
    }
}

pub(crate) fn mouse_press(button: Button) {
    unsafe {
        let dpy = open_display();
        XTestFakeButtonEvent(dpy, button_number(button), 1, 0);
        XFlush(dpy);
        close_display(dpy);
    }
}

pub(crate) fn mouse_release(button: Button) {
    unsafe {
        let dpy = open_display();
        XTestFakeButtonEvent(dpy, button_number(button), 0, 0);
        XFlush(dpy);
        close_display(dpy);
    }
}

fn scroll_click(dpy: *mut c_void, button: c_uint) {
    unsafe {
        XTestFakeButtonEvent(dpy, button, 1, 0);
        XTestFakeButtonEvent(dpy, button, 0, 0);
    }
}

pub(crate) fn mouse_scroll_vertical(lines: i32) {
    unsafe {
        let dpy = open_display();
        // 4 = up, 5 = down.
        let btn = if lines >= 0 { 4 } else { 5 };
        for _ in 0..lines.abs() {
            scroll_click(dpy, btn);
        }
        XFlush(dpy);
        close_display(dpy);
    }
}

pub(crate) fn mouse_scroll_horizontal(lines: i32) {
    unsafe {
        let dpy = open_display();
        // 7 = right, 6 = left.
        let btn = if lines >= 0 { 7 } else { 6 };
        for _ in 0..lines.abs() {
            scroll_click(dpy, btn);
        }
        XFlush(dpy);
        close_display(dpy);
    }
}

// ---------------------------------------------------------------------------
// Keyboard
// ---------------------------------------------------------------------------

pub(crate) fn key_press(key: Key) {
    match key {
        Key::MouseLeft => return mouse_press(Button::Left),
        Key::MouseRight => return mouse_press(Button::Right),
        Key::MouseMiddle => return mouse_press(Button::Middle),
        Key::MouseX1 => return mouse_press(Button::X1),
        Key::MouseX2 => return mouse_press(Button::X2),
        _ => {}
    }
    unsafe {
        let dpy = open_display();
        let ks = key_to_keysym(key);
        let kc = keysym_to_keycode(dpy, ks, &format!("{:?}", key));
        fake_key(dpy, kc, true);
        close_display(dpy);
    }
}

pub(crate) fn key_release(key: Key) {
    match key {
        Key::MouseLeft => return mouse_release(Button::Left),
        Key::MouseRight => return mouse_release(Button::Right),
        Key::MouseMiddle => return mouse_release(Button::Middle),
        Key::MouseX1 => return mouse_release(Button::X1),
        Key::MouseX2 => return mouse_release(Button::X2),
        _ => {}
    }
    unsafe {
        let dpy = open_display();
        let ks = key_to_keysym(key);
        let kc = keysym_to_keycode(dpy, ks, &format!("{:?}", key));
        fake_key(dpy, kc, false);
        close_display(dpy);
    }
}

pub(crate) fn char_press(c: char) {
    unsafe {
        let dpy = open_display();
        let (ks, shift) = char_keysym_and_shift(c);
        let kc = keysym_to_keycode(dpy, ks, &format!("char {:?}", c));
        if shift {
            fake_key(dpy, shift_keycode(dpy), true);
        }
        fake_key(dpy, kc, true);
        // NOTE: shift is intentionally left held; char_release undoes it.
        close_display(dpy);
    }
}

pub(crate) fn char_release(c: char) {
    unsafe {
        let dpy = open_display();
        let (ks, shift) = char_keysym_and_shift(c);
        let kc = keysym_to_keycode(dpy, ks, &format!("char {:?}", c));
        fake_key(dpy, kc, false);
        if shift {
            fake_key(dpy, shift_keycode(dpy), false);
        }
        close_display(dpy);
    }
}

pub(crate) fn char_tap(c: char) {
    unsafe {
        let dpy = open_display();
        let (ks, shift) = char_keysym_and_shift(c);
        let kc = keysym_to_keycode(dpy, ks, &format!("char {:?}", c));
        if shift {
            fake_key(dpy, shift_keycode(dpy), true);
        }
        fake_key(dpy, kc, true);
        fake_key(dpy, kc, false);
        if shift {
            fake_key(dpy, shift_keycode(dpy), false);
        }
        close_display(dpy);
    }
}

pub(crate) fn typewrite_str(s: &str) {
    for c in s.chars() {
        char_tap(c);
    }
}

pub(crate) fn key_is_down(key: Key) -> bool {
    // Mouse-as-key variants report the pointer button mask instead.
    if matches!(
        key,
        Key::MouseLeft | Key::MouseRight | Key::MouseMiddle | Key::MouseX1 | Key::MouseX2
    ) {
        return false;
    }
    unsafe {
        let dpy = open_display();
        let ks = key_to_keysym(key);
        let kc = XKeysymToKeycode(dpy, ks) as c_int;
        if kc == 0 {
            close_display(dpy);
            return false;
        }
        let mut keys = [0 as c_char; 32];
        XQueryKeymap(dpy, keys.as_mut_ptr());
        close_display(dpy);
        let kc = kc as usize;
        let byte = kc / 8;
        let bit = kc % 8;
        if byte >= 32 {
            return false;
        }
        (keys[byte] as u8 & (1 << bit)) != 0
    }
}

// ---------------------------------------------------------------------------
// Screen
// ---------------------------------------------------------------------------

pub(crate) fn screen_size() -> (u16, u16) {
    unsafe {
        let dpy = open_display();
        let screen = XDefaultScreen(dpy);
        let w = XDisplayWidth(dpy, screen);
        let h = XDisplayHeight(dpy, screen);
        close_display(dpy);
        if w <= 0 || h <= 0 {
            panic!("rsautogui: unable to get screen size");
        }
        (w as u16, h as u16)
    }
}

#[inline]
fn channel_value(pixel: u64, mask: u64) -> u8 {
    if mask == 0 {
        return 0;
    }
    let shift = mask.trailing_zeros();
    let max = (mask >> shift) as u64;
    if max == 0 {
        return 0;
    }
    let v = ((pixel & mask) >> shift) as u64;
    ((v * 255) / max) as u8
}

pub(crate) fn screenshot_capture(x: u16, y: u16, width: u16, height: u16) -> Image {
    if width == 0 || height == 0 {
        panic!("rsautogui: screenshot dimensions must be non-zero");
    }
    unsafe {
        let dpy = open_display();
        let root = XDefaultRootWindow(dpy);
        let img = XGetImage(
            dpy,
            root,
            x as c_int,
            y as c_int,
            width as c_uint,
            height as c_uint,
            c_ulong::MAX,
            ZPIXMAP,
        );
        if img.is_null() {
            close_display(dpy);
            panic!("rsautogui: XGetImage failed");
        }
        let w = (*img).width as usize;
        let h = (*img).height as usize;
        let bpl = (*img).bytes_per_line as usize;
        let bpp = (*img).bits_per_pixel as usize;
        let order = (*img).byte_order;
        let rm = (*img).red_mask as u64;
        let gm = (*img).green_mask as u64;
        let bm = (*img).blue_mask as u64;
        let data = (*img).data as *const u8;

        if bpp != 24 && bpp != 32 {
            let bpp_copy = bpp;
            XDestroyImage(img);
            close_display(dpy);
            panic!(
                "rsautogui: unsupported XImage depth ({}bpp); expected 24 or 32",
                bpp_copy
            );
        }

        let bpx = bpp / 8;
        let mut out = vec![0u8; w * h * 4];
        for row in 0..h {
            for col in 0..w {
                let src = data.add(row * bpl + col * bpx);
                let pixel: u64 = if bpx == 4 {
                    let b = [ *src, *src.add(1), *src.add(2), *src.add(3) ];
                    if order == LSB_FIRST {
                        (b[0] as u64) | ((b[1] as u64) << 8) | ((b[2] as u64) << 16) | ((b[3] as u64) << 24)
                    } else {
                        (b[3] as u64) | ((b[2] as u64) << 8) | ((b[1] as u64) << 16) | ((b[0] as u64) << 24)
                    }
                } else {
                    let b = [*src, *src.add(1), *src.add(2)];
                    if order == LSB_FIRST {
                        (b[0] as u64) | ((b[1] as u64) << 8) | ((b[2] as u64) << 16)
                    } else {
                        (b[2] as u64) | ((b[1] as u64) << 8) | ((b[0] as u64) << 16)
                    }
                };
                let i = (row * w + col) * 4;
                out[i] = channel_value(pixel, rm);
                out[i + 1] = channel_value(pixel, gm);
                out[i + 2] = channel_value(pixel, bm);
                out[i + 3] = 255;
            }
        }

        XDestroyImage(img);
        close_display(dpy);
        Image::from_rgba(w as u32, h as u32, out)
    }
}

// ---------------------------------------------------------------------------
// Non-panicking probes (used by the Wayland backend to reuse XWayland).
// ---------------------------------------------------------------------------

/// Returns true if an X display can be opened (X11 or XWayland running).
pub(crate) fn display_available() -> bool {
    unsafe {
        let dpy = XOpenDisplay(null());
        if dpy.is_null() {
            return false;
        }
        close_display(dpy);
        true
    }
}

/// Mouse position via X, or `None` when no X display is reachable.
pub(crate) fn try_mouse_position() -> Option<(u16, u16)> {
    unsafe {
        let dpy = XOpenDisplay(null());
        if dpy.is_null() {
            return None;
        }
        let root = XDefaultRootWindow(dpy);
        let (mut root_ret, mut child_ret) = (0, 0);
        let (mut rx, mut ry, mut wx, mut wy) = (0, 0, 0, 0);
        let mut mask = 0;
        let ok = XQueryPointer(
            dpy, root, &mut root_ret, &mut child_ret, &mut rx, &mut ry, &mut wx, &mut wy,
            &mut mask,
        );
        close_display(dpy);
        if ok == 0 {
            return None;
        }
        Some((rx.max(0) as u16, ry.max(0) as u16))
    }
}

/// Screen size via X, or `None` when no X display is reachable.
pub(crate) fn try_screen_size() -> Option<(u16, u16)> {
    unsafe {
        let dpy = XOpenDisplay(null());
        if dpy.is_null() {
            return None;
        }
        let screen = XDefaultScreen(dpy);
        let w = XDisplayWidth(dpy, screen);
        let h = XDisplayHeight(dpy, screen);
        close_display(dpy);
        if w <= 0 || h <= 0 {
            return None;
        }
        Some((w as u16, h as u16))
    }
}
