//! # rsautogui
//!
//! `rsautogui` is a cross-platform GUI automation crate.
//! It lets you control the mouse and keyboard to automate interactions with
//! other applications, and to inspect the screen.
//!
//! This crate takes heavy inspiration from [PyAutoGUI](https://github.com/asweigart/pyautogui).
//!
//! ## Platform support
//!
//! * Windows (Win32 `SendInput` / GDI, no extra setup).
//! * Linux with X11 (`Xlib` + `XTest`). No cargo dependencies — the crate
//!   links directly against the system `libX11` / `libXtst`.
//! * Linux with Wayland, natively: input goes through XWayland when one is
//!   running, otherwise through a kernel `uinput` device (`/dev/uinput`
//!   access required); screenshots go through `xdg-desktop-portal` (with a
//!   GNOME Shell fallback). The backend is picked from the session
//!   (`XDG_SESSION_TYPE` / `WAYLAND_DISPLAY`) and can be forced with
//!   `RSAUTOGUI_BACKEND=x11|wayland`.
//! * macOS (`CoreGraphics`). Note: the process needs accessibility /
//!   screen-recording permission for input injection and screenshots.
//!
//! ## Zero dependencies
//!
//! Everything is implemented with direct OS FFI plus `std`. Screenshots are
//! returned as [`image::Image`] (RGBA8) and can be saved as BMP or PNG
//! without any third-party image crate.
//!
//! ## Example
//!
//! ```no_run
//! use rsautogui::{mouse, keyboard, screen};
//! use rsautogui::mouse::{Button, Speed};
//! use rsautogui::keyboard::Key;
//!
//! fn main() {
//!     let pos = mouse::position();
//!     mouse::move_to(500, 500);
//!     mouse::slow_move_to(600, 600, Speed::Fast);
//!     mouse::click(Button::Left);
//!
//!     keyboard::typewrite("Hello, world!");
//!     keyboard::key_tap(Key::Enter);
//!
//!     let (w, h) = screen::size();
//!     let shot = screen::screenshot(0, 0, w, h);
//!     screen::printscreen(shot, "/tmp/shot.png");
//! }
//! ```

pub mod image;
pub mod keyboard;
pub mod mouse;
pub(crate) mod platform;
pub mod screen;
