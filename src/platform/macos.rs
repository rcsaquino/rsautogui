//! macOS backend: CoreGraphics + CoreFoundation. No third-party code.
//!
//! Note: input injection needs Accessibility permission and screenshots
//! need Screen Recording permission for the calling process.
#![allow(clippy::all)]

use std::os::raw::{c_void, c_int};
use std::ptr::null;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::image::Image;
use crate::keyboard::Key;
use crate::mouse::Button;

// ---------------------------------------------------------------------------
// CoreGraphics types
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct CGPoint {
    x: f64,
    y: f64,
}

// ---------------------------------------------------------------------------
// FFI
// ---------------------------------------------------------------------------

// Event types (CGEventType).
const K_LEFT_DOWN: u32 = 1;
const K_LEFT_UP: u32 = 2;
const K_RIGHT_DOWN: u32 = 3;
const K_RIGHT_UP: u32 = 4;
const K_MOUSE_MOVED: u32 = 5;
const K_LEFT_DRAGGED: u32 = 6;
const K_RIGHT_DRAGGED: u32 = 7;
const K_OTHER_DOWN: u32 = 14;
const K_OTHER_UP: u32 = 15;
const K_OTHER_DRAGGED: u32 = 16;

const K_HID_TAP: u32 = 0;

const BTN_LEFT: u32 = 0;
const BTN_RIGHT: u32 = 1;
const BTN_CENTER: u32 = 2;

const SCROLL_UNIT_LINE: u32 = 1;

const HID_SYSTEM_STATE: i32 = 1;

#[link(name = "CoreGraphics", kind = "framework")]
#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CGMainDisplayID() -> u32;
    fn CGDisplayPixelsWide(display: u32) -> usize;
    fn CGDisplayPixelsHigh(display: u32) -> usize;
    fn CGWarpMouseCursorPosition(point: CGPoint) -> i32;
    fn CGEventCreate(source: *const c_void) -> *mut c_void;
    fn CGEventGetLocation(event: *mut c_void) -> CGPoint;
    fn CFRelease(cf: *const c_void);
    fn CGEventCreateMouseEvent(
        source: *const c_void,
        mouse_type: u32,
        mouse_cursor_position: CGPoint,
        mouse_button: u32,
    ) -> *mut c_void;
    fn CGEventPost(tap: u32, event: *mut c_void);
    fn CGEventCreateKeyboardEvent(
        source: *const c_void,
        virtual_key: u16,
        key_down: bool,
    ) -> *mut c_void;
    fn CGEventKeyboardSetUnicodeString(
        event: *mut c_void,
        string_length: u32,
        unicode_string: *const u16,
    );
    fn CGEventSourceKeyState(state: i32, key: u16) -> bool;
    fn CGEventCreateScrollWheelEvent(
        source: *const c_void,
        units: u32,
        wheel_count: u32,
        wheel1: i32,
        ...
    ) -> *mut c_void;
    fn CGDisplayCreateImage(display_id: u32) -> *mut c_void;
    fn CGImageGetWidth(image: *const c_void) -> usize;
    fn CGImageGetHeight(image: *const c_void) -> usize;
    fn CGImageGetBytesPerRow(image: *const c_void) -> usize;
    fn CGImageGetBitsPerPixel(image: *const c_void) -> usize;
    fn CGImageGetBitmapInfo(image: *const c_void) -> u32;
    fn CGImageGetDataProvider(image: *const c_void) -> *mut c_void;
    fn CGImageRelease(image: *mut c_void);
    fn CGDataProviderCopyData(provider: *const c_void) -> *mut c_void;
    fn CFDataGetLength(data: *const c_void) -> isize;
    fn CFDataGetBytePtr(data: *const c_void) -> *const u8;
}

// ---------------------------------------------------------------------------
// Button-held tracking so moves while dragging emit drag events.
// ---------------------------------------------------------------------------

static LEFT_HELD: AtomicBool = AtomicBool::new(false);
static RIGHT_HELD: AtomicBool = AtomicBool::new(false);
static OTHER_HELD: AtomicBool = AtomicBool::new(false);

fn post_mouse(kind: u32, pos: CGPoint, button: u32) {
    unsafe {
        let ev = CGEventCreateMouseEvent(null(), kind, pos, button);
        if ev.is_null() {
            panic!("rsautogui: CGEventCreateMouseEvent failed");
        }
        CGEventPost(K_HID_TAP, ev);
        CFRelease(ev);
    }
}

fn current_point() -> CGPoint {
    unsafe {
        let ev = CGEventCreate(null());
        if ev.is_null() {
            panic!("rsautogui: CGEventCreate failed");
        }
        let pt = CGEventGetLocation(ev);
        CFRelease(ev);
        pt
    }
}

fn button_kind_button(button: Button, pressed: bool) -> (u32, u32) {
    match button {
        Button::Left => (if pressed { K_LEFT_DOWN } else { K_LEFT_UP }, BTN_LEFT),
        Button::Right => (if pressed { K_RIGHT_DOWN } else { K_RIGHT_UP }, BTN_RIGHT),
        Button::Middle => (if pressed { K_OTHER_DOWN } else { K_OTHER_UP }, BTN_CENTER),
        // Extra buttons have no first-class CGMouseButton; use Other with
        // extended button numbers (best effort).
        Button::X1 => (if pressed { K_OTHER_DOWN } else { K_OTHER_UP }, 3),
        Button::X2 => (if pressed { K_OTHER_DOWN } else { K_OTHER_UP }, 4),
    }
}

// ---------------------------------------------------------------------------
// Keycode mapping
// ---------------------------------------------------------------------------

/// Maps a [`Key`] to a macOS virtual keycode, or `None` if unsupported.
fn key_to_keycode_opt(key: Key) -> Option<u16> {
    use Key::*;
    let code: u16 = match key {
        A => 0,
        S => 1,
        D => 2,
        F => 3,
        H => 4,
        G => 5,
        Z => 6,
        X => 7,
        C => 8,
        V => 9,
        B => 11,
        Q => 12,
        W => 13,
        E => 14,
        R => 15,
        Y => 16,
        T => 17,
        _1 => 18,
        _2 => 19,
        _3 => 20,
        _4 => 21,
        _6 => 22,
        _5 => 23,
        Plus => 24, // '=' key (VK_OEM_PLUS)
        _9 => 25,
        _7 => 26,
        Minus => 27, // '-' key
        _8 => 28,
        _0 => 29,
        Oem6 => 30, // ']'
        O => 31,
        U => 32,
        Oem4 => 33, // '['
        I => 34,
        P => 35,
        Enter => 36,
        L => 37,
        J => 38,
        Oem7 => 39, // '\''
        K => 40,
        Oem1 => 41, // ';'
        Oem5 => 42, // '\'
        Comma => 43,
        Oem2 => 44, // '/'
        N => 45,
        M => 46,
        Period => 47,
        Tab => 48,
        Space => 49,
        Oem3 => 50, // '`'
        Backspace => 51,
        Escape => 53,
        RightWin => 54, // Right Command
        LeftWin => 55,  // Command
        Shift | LeftShift => 56,
        CapsLock => 57,
        Alt | LeftMenu => 58, // Option
        Control | LeftControl => 59,
        RightShift => 60,
        RightMenu => 61, // Right Option
        RightControl => 62,
        F17 => 64,
        Decimal => 65,   // keypad '.'
        Separator => 65, // keypad separator: closest is '.'
        Multiply => 67,  // keypad '*'
        Add => 69,       // keypad '+'
        Clear => 71,     // keypad Clear
        Numlock => 71,   // keypad Clear doubles as Num Lock
        VolumeUp => 72,
        VolumeDown => 73,
        VolumeMute => 74,
        Divide => 75, // keypad '/'
        F18 => 79,
        F19 => 80,
        Numpad0 => 82,
        Numpad1 => 83,
        Numpad2 => 84,
        Numpad3 => 85,
        Numpad4 => 86,
        Numpad5 => 87,
        Numpad6 => 88,
        Numpad7 => 89,
        Numpad8 => 91,
        Numpad9 => 92,
        F5 => 96,
        F6 => 97,
        PrevTrack => 98, // F7 / prev
        F3 => 99,
        MediaPlayPause => 100, // F8 / play
        NextTrack => 101,      // F9 / next
        F11 => 103,
        F13 => 105,
        F16 => 106,
        F14 => 107,
        F10 => 109,
        F12 => 111,
        F15 => 113,
        Insert => 114, // Help
        Home => 115,
        PageUp => 116,
        Delete => 117, // Forward delete
        F4 => 118,
        End => 119,
        F2 => 120,
        PageDown => 121,
        F1 => 122,
        LeftArrow => 123,
        RightArrow => 124,
        DownArrow => 125,
        UpArrow => 126,
        F20 => 90,
        Subtract => 78, // keypad '-'
        Oem102 => 10,   // ISO extra key (§/± on ISO boards)
        // Anything else has no macOS equivalent.
        _ => return None,
    };
    Some(code)
}

fn key_to_keycode(key: Key) -> u16 {
    match key_to_keycode_opt(key) {
        Some(c) => c,
        None => panic!("rsautogui: key {:?} is not supported on macOS", key),
    }
}

// ---------------------------------------------------------------------------
// Mouse
// ---------------------------------------------------------------------------

pub(crate) fn mouse_position() -> (u16, u16) {
    let pt = current_point();
    (
        pt.x.max(0.0).round() as u16,
        pt.y.max(0.0).round() as u16,
    )
}

pub(crate) fn mouse_set_position(x: u16, y: u16) {
    let pt = CGPoint {
        x: x as f64,
        y: y as f64,
    };
    unsafe {
        // Warp first so the cursor actually moves, then post the matching
        // event so apps observe it (drag variant when a button is held).
        if CGWarpMouseCursorPosition(pt) != 0 {
            panic!("rsautogui: unable to move mouse to ({}, {})", x, y);
        }
        if LEFT_HELD.load(Ordering::SeqCst) {
            post_mouse(K_LEFT_DRAGGED, pt, BTN_LEFT);
        } else if RIGHT_HELD.load(Ordering::SeqCst) {
            post_mouse(K_RIGHT_DRAGGED, pt, BTN_RIGHT);
        } else if OTHER_HELD.load(Ordering::SeqCst) {
            post_mouse(K_OTHER_DRAGGED, pt, BTN_CENTER);
        } else {
            post_mouse(K_MOUSE_MOVED, pt, BTN_LEFT);
        }
    }
}

pub(crate) fn mouse_move_relative(dx: i32, dy: i32) {
    let cur = current_point();
    let nx = (cur.x as i32 + dx).max(0) as u16;
    let ny = (cur.y as i32 + dy).max(0) as u16;
    mouse_set_position(nx, ny);
}

pub(crate) fn mouse_press(button: Button) {
    let pt = current_point();
    let (kind, btn) = button_kind_button(button, true);
    match button {
        Button::Left => LEFT_HELD.store(true, Ordering::SeqCst),
        Button::Right => RIGHT_HELD.store(true, Ordering::SeqCst),
        _ => OTHER_HELD.store(true, Ordering::SeqCst),
    }
    post_mouse(kind, pt, btn);
}

pub(crate) fn mouse_release(button: Button) {
    let pt = current_point();
    let (kind, btn) = button_kind_button(button, false);
    match button {
        Button::Left => LEFT_HELD.store(false, Ordering::SeqCst),
        Button::Right => RIGHT_HELD.store(false, Ordering::SeqCst),
        _ => OTHER_HELD.store(false, Ordering::SeqCst),
    }
    post_mouse(kind, pt, btn);
}

pub(crate) fn mouse_scroll_vertical(lines: i32) {
    unsafe {
        let ev = CGEventCreateScrollWheelEvent(null(), SCROLL_UNIT_LINE, 1, lines);
        if ev.is_null() {
            panic!("rsautogui: CGEventCreateScrollWheelEvent failed");
        }
        CGEventPost(K_HID_TAP, ev);
        CFRelease(ev);
    }
}

pub(crate) fn mouse_scroll_horizontal(lines: i32) {
    unsafe {
        // wheelCount = 2: wheel1 = vertical (0), wheel2 = horizontal.
        let ev = CGEventCreateScrollWheelEvent(null(), SCROLL_UNIT_LINE, 2, 0, lines);
        if ev.is_null() {
            panic!("rsautogui: CGEventCreateScrollWheelEvent failed");
        }
        CGEventPost(K_HID_TAP, ev);
        CFRelease(ev);
    }
}

// ---------------------------------------------------------------------------
// Keyboard
// ---------------------------------------------------------------------------

fn post_keycode(code: u16, down: bool) {
    unsafe {
        let ev = CGEventCreateKeyboardEvent(null(), code, down);
        if ev.is_null() {
            panic!("rsautogui: CGEventCreateKeyboardEvent failed");
        }
        CGEventPost(K_HID_TAP, ev);
        CFRelease(ev);
    }
}

fn post_unicode(c: char, down: bool) {
    let mut buf = [0u16; 2];
    let units = c.encode_utf16(&mut buf);
    unsafe {
        // Keycode 0 is a placeholder; the unicode string carries the meaning.
        let ev = CGEventCreateKeyboardEvent(null(), 0, down);
        if ev.is_null() {
            panic!("rsautogui: CGEventCreateKeyboardEvent failed");
        }
        CGEventKeyboardSetUnicodeString(ev, units.len() as u32, units.as_ptr());
        CGEventPost(K_HID_TAP, ev);
        CFRelease(ev);
    }
}

pub(crate) fn key_press(key: Key) {
    match key {
        Key::MouseLeft => return mouse_press(Button::Left),
        Key::MouseRight => return mouse_press(Button::Right),
        Key::MouseMiddle => return mouse_press(Button::Middle),
        Key::MouseX1 => return mouse_press(Button::X1),
        Key::MouseX2 => return mouse_press(Button::X2),
        _ => {}
    }
    post_keycode(key_to_keycode(key), true);
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
    post_keycode(key_to_keycode(key), false);
}

pub(crate) fn char_press(c: char) {
    post_unicode(c, true);
}

pub(crate) fn char_release(c: char) {
    post_unicode(c, false);
}

pub(crate) fn char_tap(c: char) {
    post_unicode(c, true);
    post_unicode(c, false);
}

pub(crate) fn typewrite_str(s: &str) {
    for c in s.chars() {
        char_tap(c);
    }
}

pub(crate) fn key_is_down(key: Key) -> bool {
    match key_to_keycode_opt(key) {
        Some(code) => unsafe { CGEventSourceKeyState(HID_SYSTEM_STATE, code) },
        None => false,
    }
}

// ---------------------------------------------------------------------------
// Screen
// ---------------------------------------------------------------------------

pub(crate) fn screen_size() -> (u16, u16) {
    unsafe {
        let display = CGMainDisplayID();
        let w = CGDisplayPixelsWide(display);
        let h = CGDisplayPixelsHigh(display);
        if w == 0 || h == 0 {
            panic!("rsautogui: unable to get screen size");
        }
        (w.min(u16::MAX as usize) as u16, h.min(u16::MAX as usize) as u16)
    }
}

/// Converts one CGImage pixel (4 bytes) to RGBA using the image bitmap info.
fn convert_pixel(b0: u8, b1: u8, b2: u8, b3: u8, info: u32) -> [u8; 4] {
    const BYTE_ORDER_MASK: u32 = 0x7000;
    const BYTE_ORDER_DEFAULT: u32 = 0x0000;
    const BYTE_ORDER_32_LITTLE: u32 = 0x2000;
    const BYTE_ORDER_32_BIG: u32 = 0x4000;
    const ALPHA_MASK: u32 = 0x1F;
    const ALPHA_FIRST: u32 = 2;
    const ALPHA_LAST: u32 = 1;

    // Normalize to big-endian word order: little-endian pixels are reversed.
    let order = info & BYTE_ORDER_MASK;
    let (a0, a1, a2, a3) = match order {
        BYTE_ORDER_32_LITTLE => (b3, b2, b1, b0),
        BYTE_ORDER_DEFAULT | BYTE_ORDER_32_BIG => (b0, b1, b2, b3),
        // 16-bit orders never occur for 32bpp captures; treat as big.
        _ => (b0, b1, b2, b3),
    };
    let alpha = info & ALPHA_MASK;
    // First => [A,R,G,B]; Last/Skip/None => [R,G,B,A-or-opaque].
    let first = alpha == ALPHA_FIRST
        || alpha == 4 // kCGImageAlphaFirst
        || alpha == 6; // kCGImageAlphaNoneSkipFirst
    if first {
        let a = if alpha == 6 { 255 } else { a0 };
        [a1, a2, a3, a]
    } else {
        let a = if alpha == 5 // kCGImageAlphaNoneSkipLast
            || alpha == 0 // kCGImageAlphaNone
        {
            255
        } else {
            a3
        };
        // Silence unused warning for ALPHA_LAST on some toolchains.
        let _ = ALPHA_LAST;
        [a0, a1, a2, a]
    }
}

pub(crate) fn screenshot_capture(x: u16, y: u16, width: u16, height: u16) -> Image {
    if width == 0 || height == 0 {
        panic!("rsautogui: screenshot dimensions must be non-zero");
    }
    unsafe {
        let display = CGMainDisplayID();
        let img = CGDisplayCreateImage(display);
        if img.is_null() {
            panic!(
                "rsautogui: CGDisplayCreateImage failed (missing Screen Recording permission?)"
            );
        }
        let full_w = CGImageGetWidth(img);
        let full_h = CGImageGetHeight(img);
        let bpr = CGImageGetBytesPerRow(img);
        let bpp = CGImageGetBitsPerPixel(img);
        let info = CGImageGetBitmapInfo(img);

        if bpp != 32 {
            CGImageRelease(img);
            panic!(
                "rsautogui: unsupported CGImage depth ({}bpp); expected 32",
                bpp
            );
        }
        if (x as usize) + (width as usize) > full_w
            || (y as usize) + (height as usize) > full_h
        {
            CGImageRelease(img);
            panic!("rsautogui: screenshot region is outside the display");
        }

        let provider = CGImageGetDataProvider(img);
        if provider.is_null() {
            CGImageRelease(img);
            panic!("rsautogui: CGImageGetDataProvider failed");
        }
        let data = CGDataProviderCopyData(provider);
        if data.is_null() {
            CGImageRelease(img);
            panic!("rsautogui: CGDataProviderCopyData failed");
        }
        let len = CFDataGetLength(data) as usize;
        let ptr = CFDataGetBytePtr(data);
        if ptr.is_null() {
            CFRelease(data);
            CGImageRelease(img);
            panic!("rsautogui: CFDataGetBytePtr failed");
        }
        let src = std::slice::from_raw_parts(ptr, len);

        // Provider rows are top-down with `bpr` stride.
        let mut out = vec![0u8; width as usize * height as usize * 4];
        for row in 0..height as usize {
            for col in 0..width as usize {
                let sx = x as usize + col;
                let sy = y as usize + row;
                let off = sy * bpr + sx * 4;
                if off + 4 > src.len() {
                    CFRelease(data);
                    CGImageRelease(img);
                    panic!("rsautogui: CGImage data shorter than expected");
                }
                let px = convert_pixel(src[off], src[off + 1], src[off + 2], src[off + 3], info);
                let i = (row * width as usize + col) * 4;
                out[i..i + 4].copy_from_slice(&px);
            }
        }

        CFRelease(data);
        CGImageRelease(img);
        Image::from_rgba(width as u32, height as u32, out)
    }
}

// Silence unused-import warning on some toolchains.
#[allow(dead_code)]
fn _use_c_int(_: c_int) {}
