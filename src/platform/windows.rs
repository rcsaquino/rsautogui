//! Windows backend: Win32 `SendInput` + GDI. No third-party code.
#![allow(clippy::all)]

use std::mem::{size_of, zeroed};
use std::os::raw::{c_int, c_uint, c_void};
use std::ptr::null_mut;

use crate::image::Image;
use crate::keyboard::Key;
use crate::mouse::Button;

// ---------------------------------------------------------------------------
// FFI declarations (user32 / gdi32)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct POINT {
    x: i32,
    y: i32,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct MOUSEINPUT {
    dx: i32,
    dy: i32,
    mouse_data: u32,
    dw_flags: u32,
    time: u32,
    dw_extra_info: usize,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct KEYBDINPUT {
    w_vk: u16,
    w_scan: u16,
    dw_flags: u32,
    time: u32,
    dw_extra_info: usize,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct HARDWAREINPUT {
    u_msg: u32,
    w_param_l: u16,
    w_param_h: u16,
}

#[repr(C)]
union INPUT_UNION {
    mi: MOUSEINPUT,
    ki: KEYBDINPUT,
    hi: HARDWAREINPUT,
}

// INPUT contains a union, so Copy must be manual.
impl Copy for INPUT {}
impl Clone for INPUT {
    fn clone(&self) -> Self {
        *self
    }
}

#[repr(C)]
struct INPUT {
    type_: u32,
    u: INPUT_UNION,
}

#[repr(C)]
struct BITMAPINFOHEADER {
    bi_size: u32,
    bi_width: i32,
    bi_height: i32,
    bi_planes: u16,
    bi_bit_count: u16,
    bi_compression: u32,
    bi_size_image: u32,
    bi_x_pels_per_meter: i32,
    bi_y_pels_per_meter: i32,
    bi_clr_used: u32,
    bi_clr_important: u32,
}

#[repr(C)]
struct BITMAPINFO {
    header: BITMAPINFOHEADER,
    // Space for color masks (unused for 32-bit BI_RGB, keeps layout valid).
    colors: [u32; 3],
}

const INPUT_MOUSE: u32 = 0;
const INPUT_KEYBOARD: u32 = 1;

const MOUSEEVENTF_LEFTDOWN: u32 = 0x0002;
const MOUSEEVENTF_LEFTUP: u32 = 0x0004;
const MOUSEEVENTF_RIGHTDOWN: u32 = 0x0008;
const MOUSEEVENTF_RIGHTUP: u32 = 0x0010;
const MOUSEEVENTF_MIDDLEDOWN: u32 = 0x0020;
const MOUSEEVENTF_MIDDLEUP: u32 = 0x0040;
const MOUSEEVENTF_XDOWN: u32 = 0x0080;
const MOUSEEVENTF_XUP: u32 = 0x0100;
const MOUSEEVENTF_WHEEL: u32 = 0x0800;
const MOUSEEVENTF_HWHEEL: u32 = 0x1000;

const XBUTTON1: u32 = 0x0001;
const XBUTTON2: u32 = 0x0002;

const KEYEVENTF_EXTENDEDKEY: u32 = 0x0001;
const KEYEVENTF_KEYUP: u32 = 0x0002;
const KEYEVENTF_UNICODE: u32 = 0x0004;

const SM_CXSCREEN: c_int = 0;
const SM_CYSCREEN: c_int = 1;

const SRCCOPY: u32 = 0x00CC_0020;
const BI_RGB: u32 = 0;
const DIB_RGB_COLORS: c_uint = 0;
const WHEEL_DELTA: i32 = 120;

#[link(name = "user32")]
extern "system" {
    fn GetCursorPos(lp_point: *mut POINT) -> c_int;
    fn SetCursorPos(x: c_int, y: c_int) -> c_int;
    fn SendInput(n_inputs: c_uint, p_inputs: *const INPUT, cb_size: c_int) -> c_uint;
    fn GetSystemMetrics(n_index: c_int) -> c_int;
    fn GetDC(h_wnd: *mut c_void) -> *mut c_void;
    fn ReleaseDC(h_wnd: *mut c_void, h_dc: *mut c_void) -> c_int;
    fn GetAsyncKeyState(v_key: c_int) -> i16;
    fn GetKeyState(n_virt_key: c_int) -> i16;
}

#[link(name = "gdi32")]
extern "system" {
    fn CreateCompatibleDC(hdc: *mut c_void) -> *mut c_void;
    fn CreateCompatibleBitmap(hdc: *mut c_void, cx: c_int, cy: c_int) -> *mut c_void;
    fn SelectObject(hdc: *mut c_void, h: *mut c_void) -> *mut c_void;
    fn BitBlt(
        hdc: *mut c_void,
        x: c_int,
        y: c_int,
        cx: c_int,
        cy: c_int,
        hdc_src: *mut c_void,
        x1: c_int,
        y1: c_int,
        rop: u32,
    ) -> c_int;
    fn GetDIBits(
        hdc: *mut c_void,
        hbm: *mut c_void,
        start: c_uint,
        c_lines: c_uint,
        lpv_bits: *mut c_void,
        lpbmi: *mut BITMAPINFO,
        usage: c_uint,
    ) -> c_int;
    fn DeleteObject(ho: *mut c_void) -> c_int;
    fn DeleteDC(hdc: *mut c_void) -> c_int;
}

// ---------------------------------------------------------------------------
// Small SendInput helpers
// ---------------------------------------------------------------------------

fn send_mouse(flags: u32, data: u32) {
    unsafe {
        let input = INPUT {
            type_: INPUT_MOUSE,
            u: INPUT_UNION {
                mi: MOUSEINPUT {
                    dx: 0,
                    dy: 0,
                    mouse_data: data,
                    dw_flags: flags,
                    time: 0,
                    dw_extra_info: 0,
                },
            },
        };
        let sent = SendInput(1, &input, size_of::<INPUT>() as c_int);
        if sent != 1 {
            panic!("rsautogui: SendInput (mouse) failed");
        }
    }
}

fn send_key_vk(vk: u16, down: bool) {
    let mut flags: u32 = 0;
    if is_extended_vk(vk) {
        flags |= KEYEVENTF_EXTENDEDKEY;
    }
    if !down {
        flags |= KEYEVENTF_KEYUP;
    }
    unsafe {
        let input = INPUT {
            type_: INPUT_KEYBOARD,
            u: INPUT_UNION {
                ki: KEYBDINPUT {
                    w_vk: vk,
                    w_scan: 0,
                    dw_flags: flags,
                    time: 0,
                    dw_extra_info: 0,
                },
            },
        };
        let sent = SendInput(1, &input, size_of::<INPUT>() as c_int);
        if sent != 1 {
            panic!("rsautogui: SendInput (key 0x{:02X}) failed", vk);
        }
    }
}

fn send_unicode_unit(unit: u16, down: bool) {
    let mut flags: u32 = KEYEVENTF_UNICODE;
    if !down {
        flags |= KEYEVENTF_KEYUP;
    }
    unsafe {
        let input = INPUT {
            type_: INPUT_KEYBOARD,
            u: INPUT_UNION {
                ki: KEYBDINPUT {
                    w_vk: 0,
                    w_scan: unit,
                    dw_flags: flags,
                    time: 0,
                    dw_extra_info: 0,
                },
            },
        };
        let sent = SendInput(1, &input, size_of::<INPUT>() as c_int);
        if sent != 1 {
            panic!("rsautogui: SendInput (unicode) failed");
        }
    }
}

fn is_extended_vk(vk: u16) -> bool {
    matches!(
        vk,
        0x21..=0x28 | 0x2D | 0x2E | 0x5B..=0x5D | 0xA3 | 0xA5 | 0x90 | 0x91
    )
}

fn button_down_up(button: Button) -> (u32, u32, u32) {
    match button {
        Button::Left => (MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, 0),
        Button::Right => (MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, 0),
        Button::Middle => (MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, 0),
        Button::X1 => (MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP, XBUTTON1),
        Button::X2 => (MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP, XBUTTON2),
    }
}

// ---------------------------------------------------------------------------
// Mouse
// ---------------------------------------------------------------------------

pub(crate) fn mouse_position() -> (u16, u16) {
    unsafe {
        let mut pt = POINT { x: 0, y: 0 };
        if GetCursorPos(&mut pt) == 0 {
            panic!("rsautogui: unable to locate mouse position");
        }
        (pt.x.max(0) as u16, pt.y.max(0) as u16)
    }
}

pub(crate) fn mouse_set_position(x: u16, y: u16) {
    unsafe {
        if SetCursorPos(x as c_int, y as c_int) == 0 {
            panic!("rsautogui: unable to move mouse to ({}, {})", x, y);
        }
    }
}

pub(crate) fn mouse_move_relative(dx: i32, dy: i32) {
    unsafe {
        let mut pt = POINT { x: 0, y: 0 };
        if GetCursorPos(&mut pt) == 0 {
            panic!("rsautogui: unable to locate mouse position");
        }
        if SetCursorPos(pt.x + dx, pt.y + dy) == 0 {
            panic!("rsautogui: unable to move mouse relatively");
        }
    }
}

pub(crate) fn mouse_press(button: Button) {
    let (down, _, data) = button_down_up(button);
    send_mouse(down, data);
}

pub(crate) fn mouse_release(button: Button) {
    let (_, up, data) = button_down_up(button);
    send_mouse(up, data);
}

pub(crate) fn mouse_scroll_vertical(lines: i32) {
    send_mouse(MOUSEEVENTF_WHEEL, lines.wrapping_mul(WHEEL_DELTA) as u32);
}

pub(crate) fn mouse_scroll_horizontal(lines: i32) {
    send_mouse(MOUSEEVENTF_HWHEEL, lines.wrapping_mul(WHEEL_DELTA) as u32);
}

// ---------------------------------------------------------------------------
// Keyboard
// ---------------------------------------------------------------------------

pub(crate) fn key_press(key: Key) {
    send_key_vk(key as u16, true);
}

pub(crate) fn key_release(key: Key) {
    send_key_vk(key as u16, false);
}

pub(crate) fn char_press(c: char) {
    let mut buf = [0u16; 2];
    for unit in c.encode_utf16(&mut buf).iter() {
        send_unicode_unit(*unit, true);
    }
}

pub(crate) fn char_release(c: char) {
    let mut buf = [0u16; 2];
    // Release in reverse order for surrogate pairs.
    let units: Vec<u16> = c.encode_utf16(&mut buf).iter().copied().collect();
    for unit in units.iter().rev() {
        send_unicode_unit(*unit, false);
    }
}

pub(crate) fn char_tap(c: char) {
    let mut buf = [0u16; 2];
    for unit in c.encode_utf16(&mut buf).iter() {
        send_unicode_unit(*unit, true);
        send_unicode_unit(*unit, false);
    }
}

pub(crate) fn typewrite_str(s: &str) {
    for c in s.chars() {
        char_tap(c);
    }
}

pub(crate) fn key_is_down(key: Key) -> bool {
    unsafe { (GetAsyncKeyState(key as u16 as c_int) as u16 & 0x8000) != 0 }
}

pub(crate) fn key_is_toggled(key: Key) -> bool {
    unsafe { (GetKeyState(key as u16 as c_int) as u16 & 0x0001) != 0 }
}

// ---------------------------------------------------------------------------
// Screen
// ---------------------------------------------------------------------------

pub(crate) fn screen_size() -> (u16, u16) {
    unsafe {
        let w = GetSystemMetrics(SM_CXSCREEN);
        let h = GetSystemMetrics(SM_CYSCREEN);
        if w <= 0 || h <= 0 {
            panic!("rsautogui: unable to get screen size");
        }
        (w as u16, h as u16)
    }
}

pub(crate) fn screenshot_capture(x: u16, y: u16, width: u16, height: u16) -> Image {
    if width == 0 || height == 0 {
        panic!("rsautogui: screenshot dimensions must be non-zero");
    }
    unsafe {
        let hdc_screen = GetDC(null_mut());
        if hdc_screen.is_null() {
            panic!("rsautogui: GetDC failed");
        }
        let hdc_mem = CreateCompatibleDC(hdc_screen);
        if hdc_mem.is_null() {
            ReleaseDC(null_mut(), hdc_screen);
            panic!("rsautogui: CreateCompatibleDC failed");
        }
        let hbmp = CreateCompatibleBitmap(hdc_screen, width as c_int, height as c_int);
        if hbmp.is_null() {
            DeleteDC(hdc_mem);
            ReleaseDC(null_mut(), hdc_screen);
            panic!("rsautogui: CreateCompatibleBitmap failed");
        }
        let old = SelectObject(hdc_mem, hbmp);
        let ok = BitBlt(
            hdc_mem,
            0,
            0,
            width as c_int,
            height as c_int,
            hdc_screen,
            x as c_int,
            y as c_int,
            SRCCOPY,
        );
        if ok == 0 {
            SelectObject(hdc_mem, old);
            DeleteObject(hbmp);
            DeleteDC(hdc_mem);
            ReleaseDC(null_mut(), hdc_screen);
            panic!("rsautogui: BitBlt capture failed");
        }

        let mut bmi: BITMAPINFO = zeroed();
        bmi.header.bi_size = size_of::<BITMAPINFOHEADER>() as u32;
        bmi.header.bi_width = width as i32;
        // Negative height => top-down DIB, matching our buffer order.
        bmi.header.bi_height = -(height as i32);
        bmi.header.bi_planes = 1;
        bmi.header.bi_bit_count = 32;
        bmi.header.bi_compression = BI_RGB;

        let mut buf = vec![0u8; width as usize * height as usize * 4];
        let lines = GetDIBits(
            hdc_mem,
            hbmp,
            0,
            height as c_uint,
            buf.as_mut_ptr() as *mut c_void,
            &mut bmi,
            DIB_RGB_COLORS,
        );

        SelectObject(hdc_mem, old);
        DeleteObject(hbmp);
        DeleteDC(hdc_mem);
        ReleaseDC(null_mut(), hdc_screen);

        if lines == 0 {
            panic!("rsautogui: GetDIBits failed");
        }

        // DIB 32-bit is BGRA (alpha is typically 0); convert to RGBA opaque.
        for px in buf.chunks_exact_mut(4) {
            let b = px[0];
            px[0] = px[2];
            px[2] = b;
            px[3] = 255;
        }

        Image::from_rgba(width as u32, height as u32, buf)
    }
}
