//! Native Wayland backend (used when the session is Wayland).
//!
//! Strategy, all dependency-free:
//!
//! * **Input**: X11/XTest through XWayland when a display is reachable
//!   (exact cursor control, full Unicode typing). Otherwise kernel `uinput`
//!   (`/dev/uinput`, needs access rights; US-ASCII typing only).
//! * **Position**: `XQueryPointer` when available, otherwise internally
//!   tracked (absolute moves re-sync the tracker; physical mouse movement by
//!   the user cannot be observed without XWayland).
//! * **Size**: X11 when available, otherwise the compositor's `wl_output`.
//! * **Screenshots**: `xdg-desktop-portal` (GNOME Shell D-Bus fallback).
//!   X screenshots are deliberately *not* used here: under XWayland they
//!   only contain X surfaces, with native Wayland windows missing/blank.

mod dbus;
mod portal;
mod uinput;
mod wloutput;

use std::collections::HashSet;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Mutex, OnceLock};

use crate::image::Image;
use crate::keyboard::Key;
use crate::mouse::Button;

use super::x11;
use uinput::{BTN_LEFT, BTN_MIDDLE, BTN_RIGHT, BTN_X1, BTN_X2};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn x_available() -> bool {
    static AV: OnceLock<bool> = OnceLock::new();
    *AV.get_or_init(x11::display_available)
}

static CUR_X: AtomicI32 = AtomicI32::new(0);
static CUR_Y: AtomicI32 = AtomicI32::new(0);

fn store_tracked(x: i32, y: i32) {
    CUR_X.store(x.max(0), Ordering::SeqCst);
    CUR_Y.store(y.max(0), Ordering::SeqCst);
}

fn load_tracked() -> (u16, u16) {
    let x = CUR_X.load(Ordering::SeqCst).max(0).min(u16::MAX as i32) as u16;
    let y = CUR_Y.load(Ordering::SeqCst).max(0).min(u16::MAX as i32) as u16;
    (x, y)
}

/// Re-sync the tracker from XWayland when possible.
fn sync_from_x() {
    if let Some((x, y)) = x11::try_mouse_position() {
        store_tracked(x as i32, y as i32);
    }
}

fn pressed_set() -> &'static Mutex<HashSet<u16>> {
    static S: OnceLock<Mutex<HashSet<u16>>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(HashSet::new()))
}

fn track_down(code: u16) {
    if let Ok(mut s) = pressed_set().lock() {
        s.insert(code);
    }
}

fn track_up(code: u16) {
    if let Ok(mut s) = pressed_set().lock() {
        s.remove(&code);
    }
}

fn button_code(button: Button) -> u16 {
    match button {
        Button::Left => BTN_LEFT,
        Button::Right => BTN_RIGHT,
        Button::Middle => BTN_MIDDLE,
        Button::X1 => BTN_X1,
        Button::X2 => BTN_X2,
    }
}

fn press_keycode(key: Key) {
    match key {
        Key::MouseLeft => return mouse_press(Button::Left),
        Key::MouseRight => return mouse_press(Button::Right),
        Key::MouseMiddle => return mouse_press(Button::Middle),
        Key::MouseX1 => return mouse_press(Button::X1),
        Key::MouseX2 => return mouse_press(Button::X2),
        _ => {}
    }
    if x_available() {
        x11::key_press(key);
        return;
    }
    let code = uinput::evdev_key(key);
    uinput::key_event(code, true);
    track_down(code);
}

fn release_keycode(key: Key) {
    match key {
        Key::MouseLeft => return mouse_release(Button::Left),
        Key::MouseRight => return mouse_release(Button::Right),
        Key::MouseMiddle => return mouse_release(Button::Middle),
        Key::MouseX1 => return mouse_release(Button::X1),
        Key::MouseX2 => return mouse_release(Button::X2),
        _ => {}
    }
    if x_available() {
        x11::key_release(key);
        return;
    }
    let code = uinput::evdev_key(key);
    uinput::key_event(code, false);
    track_up(code);
}

const EVDEV_SHIFT: u16 = 42;

fn press_char(c: char) {
    if x_available() {
        x11::char_press(c);
        return;
    }
    let (code, shift) = uinput::evdev_char(c);
    if shift {
        uinput::key_event(EVDEV_SHIFT, true);
        track_down(EVDEV_SHIFT);
    }
    uinput::key_event(code, true);
    track_down(code);
}

fn release_char(c: char) {
    if x_available() {
        x11::char_release(c);
        return;
    }
    let (code, shift) = uinput::evdev_char(c);
    uinput::key_event(code, false);
    track_up(code);
    if shift {
        uinput::key_event(EVDEV_SHIFT, false);
        track_up(EVDEV_SHIFT);
    }
}

fn tap_char(c: char) {
    if x_available() {
        x11::char_tap(c);
        return;
    }
    let (code, shift) = uinput::evdev_char(c);
    if shift {
        uinput::key_event(EVDEV_SHIFT, true);
    }
    uinput::key_event(code, true);
    uinput::key_event(code, false);
    if shift {
        uinput::key_event(EVDEV_SHIFT, false);
    }
}

// ---------------------------------------------------------------------------
// Mouse
// ---------------------------------------------------------------------------

pub(crate) fn mouse_position() -> (u16, u16) {
    if x_available() {
        if let Some(p) = x11::try_mouse_position() {
            store_tracked(p.0 as i32, p.1 as i32);
            return p;
        }
    }
    load_tracked()
}

pub(crate) fn mouse_set_position(x: u16, y: u16) {
    if x_available() {
        x11::mouse_set_position(x, y);
        store_tracked(x as i32, y as i32);
        return;
    }
    let dx = x as i32 - CUR_X.load(Ordering::SeqCst);
    let dy = y as i32 - CUR_Y.load(Ordering::SeqCst);
    uinput::rel_move(dx, dy);
    store_tracked(x as i32, y as i32);
}

pub(crate) fn mouse_move_relative(dx: i32, dy: i32) {
    if x_available() {
        x11::mouse_move_relative(dx, dy);
        sync_from_x();
        return;
    }
    uinput::rel_move(dx, dy);
    store_tracked(
        CUR_X.load(Ordering::SeqCst) + dx,
        CUR_Y.load(Ordering::SeqCst) + dy,
    );
}

pub(crate) fn mouse_press(button: Button) {
    if x_available() {
        x11::mouse_press(button);
        return;
    }
    uinput::key_event(button_code(button), true);
}

pub(crate) fn mouse_release(button: Button) {
    if x_available() {
        x11::mouse_release(button);
        return;
    }
    uinput::key_event(button_code(button), false);
}

pub(crate) fn mouse_scroll_vertical(lines: i32) {
    if x_available() {
        x11::mouse_scroll_vertical(lines);
        return;
    }
    uinput::scroll(true, lines);
}

pub(crate) fn mouse_scroll_horizontal(lines: i32) {
    if x_available() {
        x11::mouse_scroll_horizontal(lines);
        return;
    }
    uinput::scroll(false, lines);
}

// ---------------------------------------------------------------------------
// Keyboard
// ---------------------------------------------------------------------------

pub(crate) fn key_press(key: Key) {
    press_keycode(key);
}

pub(crate) fn key_release(key: Key) {
    release_keycode(key);
}

pub(crate) fn char_press(c: char) {
    press_char(c);
}

pub(crate) fn char_release(c: char) {
    release_char(c);
}

pub(crate) fn typewrite_str(s: &str) {
    if x_available() {
        x11::typewrite_str(s);
        return;
    }
    for c in s.chars() {
        tap_char(c);
    }
}

pub(crate) fn key_is_down(key: Key) -> bool {
    if matches!(
        key,
        Key::MouseLeft | Key::MouseRight | Key::MouseMiddle | Key::MouseX1 | Key::MouseX2
    ) {
        return false;
    }
    if x_available() {
        return x11::key_is_down(key);
    }
    match uinput::evdev_key_opt(key) {
        Some(code) => pressed_set()
            .lock()
            .map(|s| s.contains(&code))
            .unwrap_or(false),
        None => false,
    }
}

// ---------------------------------------------------------------------------
// Screen
// ---------------------------------------------------------------------------

/// Fullscreen size plus buffer scale (physical pixels per logical unit).
fn full_geometry() -> (u16, u16, u32) {
    if let Some((w, h)) = x11::try_screen_size() {
        return (w, h, 1);
    }
    match wloutput::output_info() {
        Ok((w, h, scale)) => (w, h, scale.max(1) as u32),
        Err(e) => panic!(
            "rsautogui: cannot determine screen size on Wayland (no XWayland display \
             and wl_output query failed: {e})"
        ),
    }
}

pub(crate) fn screen_size() -> (u16, u16) {
    let (w, h, _) = full_geometry();
    (w, h)
}

pub(crate) fn screenshot_capture(x: u16, y: u16, width: u16, height: u16) -> Image {
    if width == 0 || height == 0 {
        panic!("rsautogui: screenshot dimensions must be non-zero");
    }
    let (fw, fh, queried) = full_geometry();
    let full = portal::capture_fullscreen((fw, fh)).unwrap_or_else(|e| {
        panic!(
            "rsautogui: Wayland screenshot failed. Is xdg-desktop-portal running with a \
             backend for your compositor (e.g. xdg-desktop-portal-gnome/-kde/-wlr)? {e}"
        )
    });
    // The portal buffer is in physical pixels while our region is logical.
    // Derive the buffer scale from the capture itself when possible.
    let (fw32, fh32) = (fw as u32, fh as u32);
    let scale = if fw32 > 0 && fh32 > 0 && full.width() % fw32 == 0 && full.height() % fh32 == 0
    {
        let (sx, sy) = (full.width() / fw32, full.height() / fh32);
        if sx == sy && sx >= 1 {
            sx
        } else {
            queried
        }
    } else {
        queried
    };
    let (px, py, pw, ph) = (
        x as u32 * scale,
        y as u32 * scale,
        width as u32 * scale,
        height as u32 * scale,
    );
    if px.saturating_add(pw) > full.width() || py.saturating_add(ph) > full.height() {
        panic!(
            "One or more specified parameters are not within the captured screen \
             ({}x{} buffer, {}x{} logical). Use screen::size() to check. (Multi-monitor \
             region captures assume the primary output starts at the origin.)",
            full.width(),
            full.height(),
            fw,
            fh
        );
    }
    let img = full.crop(px, py, pw, ph);
    // Physical buffer -> logical pixels (matches X11/macOS/Windows, which
    // always return exactly the requested size).
    img.downscale(scale)
}
