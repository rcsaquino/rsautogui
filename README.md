# rsautogui

- `rsautogui` is a GUI automation rust crate for windows.
- It lets you control the mouse and keyboard to automate interactions with other applications.
- Currently, it's a wrapper of multiple rust crates to have pyautogui inspired syntax written in rust.

---

# Installation
Run the following Cargo command in your project directory:
```
cargo add rsautogui
```

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
use rsautogui::{keyboard, keyboard::Vk};

fn main() {
    keyboard::typewrite("Lorem ipsum!"); // Simulates typing the string provided.

    // Print `A` using virtual key `Vk`
    keyboard::key_down(Vk::Shift); // Presses specified key down.
    keyboard::key_tap(Vk::A); // Performs specified key_down and key_up.
    keyboard::key_up(Vk::Shift); // Releases specified key up.

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
    screen::printscreen(ss, "./src/assets/screenshot.jpg"); // Saves the provided screenshot to a path with the specified filename and extension.

    let loc: Option<(u16, u16)> = screen::locate_pixel(screen::Rgba([255, 255, 255, 255])); // Locates the first pixel color similar to the one specified and returns its coordinate.
    let loc: Vec<(u16, u16)> = screen::locate_all_pixel(screen::Rgba([255, 255, 255, 255])); // Locates all pixel colors similar to the one specified and returns their coordinates.
    let pixel: Rgba<u8> = screen::get_pixel(500, 500); // Get the pixel color on x, y coordinate.
}
```

---

You can read more documentation of this crate in [`docs.rs`](https://docs.rs/rsautogui/).
