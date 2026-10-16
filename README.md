# rsautogui

- `rsautogui` is a cross-platform GUI automation crate for Windows, Linux and macOS.
- It lets you control the mouse and keyboard to automate interactions with other applications.
- It has **zero dependencies**: all OS interaction is direct system FFI
  (Win32 on Windows, X11/XTest on Linux, CoreGraphics on macOS), and
  screenshots use a tiny built-in RGBA image type with BMP/PNG encoders.
- PyAutoGUI-inspired syntax written in Rust.

---

# Installation

Run the following Cargo command in your project directory:

```
cargo add rsautogui
```

No other setup is needed on Windows. Platform notes:

- **Linux (X11)**: requires `libX11` + `libXtst` on the system and a running
  X server (`$DISPLAY`). Building needs the headers too, e.g.
  `apt install libx11-dev libxtst-dev` (Debian/Ubuntu) or the equivalent.
- **Linux (Wayland)**: supported natively. Screenshots need
  `xdg-desktop-portal` with a backend for your compositor (e.g.
  `xdg-desktop-portal-gnome/-kde/-wlr`, the default on most Wayland
  desktops). Input uses XWayland automatically when one is running;
  pure-Wayland sessions (no XWayland) use a kernel `uinput` device, which
  needs `/dev/uinput` access (e.g. membership in the `input` group plus
  `KERNEL=="uinput", GROUP="input", MODE="0660"` udev rule, or root).
  Notes for pure-Wayland sessions: typing is US-ASCII only (Unicode needs
  XWayland), and `mouse::position()` is tracked internally, so it starts at
  `(0, 0)` until the first absolute move and cannot see physical mouse
  movement. Force a backend with `RSAUTOGUI_BACKEND=x11|wayland`.
- **macOS**: the app needs Accessibility permission (keyboard/mouse) and
  Screen Recording permission (screenshots).

# Example Usage

## Mouse

```rust
use rsautogui::{mouse, mouse::Speed, mouse::Button, mouse::ScrollAxis};

fn main() {
    let pos: (u16, u16) = mouse::position(); // Returns the current mouse coordinates.

    mouse::move_to(500, 500); // Moves mouse to x, y instantly.
    mouse::slow_move_to(500, 500, Speed::Faster); // Moves mouse to x, y with the specified speed.

    mouse::move_rel(500, 500); // Moves mouse to x, y relative to current position instantly.
    mouse::slow_move_rel(500, 500, Speed::Fast); // Moves mouse to x, y relative to current position with the specified speed.

    mouse::drag_to(500, 500); // Drags mouse to x, y instantly.
    mouse::slow_drag_to(500, 500, Speed::Slow); // Drags mouse to x, y with the specified speed.

    mouse::drag_rel(500, 500); // Drags mouse to x, y relative to its position instantly.
    mouse::slow_drag_rel(500, 500, Speed::Slower); // Drags mouse to x, y relative to its position with the specified speed.

    mouse::click(Button::Left); // Performs a mouse click with the specified button.
    mouse::down(Button::Left); // Performs a mouse down with the specified button.
    mouse::up(Button::Left); // Performs a mouse up with the specified button.
    mouse::scroll(ScrollAxis::Y, 10); // Scrolls x or y axis n times.
}
```

## Keyboard

```rust
use rsautogui::{keyboard, keyboard::Key};

// `Key` keeps Windows virtual-key numbers, so the old `Vk` name still works:
// `use rsautogui::keyboard::Vk;` is an alias for `Key`.

fn main() {
    keyboard::typewrite("Lorem ipsum!"); // Simulates typing the string provided.

    // Print `A` using virtual key `Key`
    keyboard::key_down(Key::Shift); // Presses specified key down.
    keyboard::key_tap(Key::A); // Performs specified key_down and key_up.
    keyboard::key_up(Key::Shift); // Releases specified key up.

    // Print `A` with one line
    keyboard::key_tap('A');
}
```

## Screen

```rust
use rsautogui::screen::{self, Rgba, DynamicImage};

fn main() {
    let size: (u16, u16) = screen::size(); // Returns the width and height of primary screen.
    let on_screen: bool = screen::on_screen(1920, 1080); // Verifies if specified x & y coordinates are present on primary screen.

    let ss: DynamicImage = screen::screenshot(0, 0, 1920, 1080); // Returns screenshot of the primary screen.
    screen::printscreen(ss, "./screenshot.png"); // Saves as .png, .bmp or .ppm (no JPEG — dependency-free).

    let loc: Option<(u16, u16)> = screen::locate_pixel(Rgba([255, 255, 255, 255])); // Locates the first pixel color equal to the one specified and returns its coordinate.
    let loc: Vec<(u16, u16)> = screen::locate_all_pixel(Rgba([255, 255, 255, 255])); // Locates all pixel colors equal to the one specified and returns their coordinates.
    let pixel: Rgba = screen::get_pixel(500, 500); // Get the pixel color on x, y coordinate.
}
```

## Documentation

You can read more documentation of this crate in [`docs.rs`](https://docs.rs/rsautogui/).

## Support and contact

If rsautogui is useful to you, you can support its development on
[Ko-fi](https://ko-fi.com/rcsaquino).

<a href='https://ko-fi.com/rcsaquino' target='_blank'><img height='72' style='border:0px;height:72px;' src='https://storage.ko-fi.com/cdn/kofi2.png?v=3' border='0' alt='Buy Me a Coffee at ko-fi.com' /></a>