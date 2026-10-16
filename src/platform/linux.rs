//! Linux backend dispatcher.
//!
//! * X11 sessions (`XDG_SESSION_TYPE=x11`, or no Wayland socket) use
//!   Xlib/XTest directly (`x11`).
//! * Wayland sessions use the native implementation (`wayland/`): input via
//!   XWayland when available, otherwise kernel `uinput`; screenshots via
//!   `xdg-desktop-portal`.
//!
//! Override the detection with `RSAUTOGUI_BACKEND=x11|wayland|auto`.

mod wayland;
mod x11;

use std::sync::OnceLock;

use crate::image::Image;
use crate::keyboard::Key;
use crate::mouse::Button;

fn detect_wayland() -> bool {
    match std::env::var("RSAUTOGUI_BACKEND") {
        Ok(v) => match v.to_ascii_lowercase().as_str() {
            "wayland" => true,
            "x11" => false,
            "auto" => auto_detect(),
            other => panic!(
                "rsautogui: unknown RSAUTOGUI_BACKEND={other:?} (expected x11, wayland or auto)"
            ),
        },
        Err(_) => auto_detect(),
    }
}

fn auto_detect() -> bool {
    let session = std::env::var("XDG_SESSION_TYPE")
        .unwrap_or_default()
        .to_ascii_lowercase();
    if session == "wayland" {
        return true;
    }
    if session == "x11" {
        return false;
    }
    // No explicit session type: Wayland socket decides.
    !std::env::var("WAYLAND_DISPLAY")
        .unwrap_or_default()
        .is_empty()
}

fn use_wayland() -> bool {
    static CACHED: OnceLock<bool> = OnceLock::new();
    *CACHED.get_or_init(detect_wayland)
}

// ---------------------------------------------------------------------------
// Mouse
// ---------------------------------------------------------------------------

pub(crate) fn mouse_position() -> (u16, u16) {
    if use_wayland() {
        wayland::mouse_position()
    } else {
        x11::mouse_position()
    }
}

pub(crate) fn mouse_set_position(x: u16, y: u16) {
    if use_wayland() {
        wayland::mouse_set_position(x, y)
    } else {
        x11::mouse_set_position(x, y)
    }
}

pub(crate) fn mouse_move_relative(dx: i32, dy: i32) {
    if use_wayland() {
        wayland::mouse_move_relative(dx, dy)
    } else {
        x11::mouse_move_relative(dx, dy)
    }
}

pub(crate) fn mouse_press(button: Button) {
    if use_wayland() {
        wayland::mouse_press(button)
    } else {
        x11::mouse_press(button)
    }
}

pub(crate) fn mouse_release(button: Button) {
    if use_wayland() {
        wayland::mouse_release(button)
    } else {
        x11::mouse_release(button)
    }
}

pub(crate) fn mouse_scroll_vertical(lines: i32) {
    if use_wayland() {
        wayland::mouse_scroll_vertical(lines)
    } else {
        x11::mouse_scroll_vertical(lines)
    }
}

pub(crate) fn mouse_scroll_horizontal(lines: i32) {
    if use_wayland() {
        wayland::mouse_scroll_horizontal(lines)
    } else {
        x11::mouse_scroll_horizontal(lines)
    }
}

// ---------------------------------------------------------------------------
// Keyboard
// ---------------------------------------------------------------------------

pub(crate) fn key_press(key: Key) {
    if use_wayland() {
        wayland::key_press(key)
    } else {
        x11::key_press(key)
    }
}

pub(crate) fn key_release(key: Key) {
    if use_wayland() {
        wayland::key_release(key)
    } else {
        x11::key_release(key)
    }
}

pub(crate) fn char_press(c: char) {
    if use_wayland() {
        wayland::char_press(c)
    } else {
        x11::char_press(c)
    }
}

pub(crate) fn char_release(c: char) {
    if use_wayland() {
        wayland::char_release(c)
    } else {
        x11::char_release(c)
    }
}

pub(crate) fn typewrite_str(s: &str) {
    if use_wayland() {
        wayland::typewrite_str(s)
    } else {
        x11::typewrite_str(s)
    }
}

pub(crate) fn key_is_down(key: Key) -> bool {
    if use_wayland() {
        wayland::key_is_down(key)
    } else {
        x11::key_is_down(key)
    }
}

// ---------------------------------------------------------------------------
// Screen
// ---------------------------------------------------------------------------

pub(crate) fn screen_size() -> (u16, u16) {
    if use_wayland() {
        wayland::screen_size()
    } else {
        x11::screen_size()
    }
}

pub(crate) fn screenshot_capture(x: u16, y: u16, width: u16, height: u16) -> Image {
    if use_wayland() {
        wayland::screenshot_capture(x, y, width, height)
    } else {
        x11::screenshot_capture(x, y, width, height)
    }
}
