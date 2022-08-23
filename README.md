# rsautogui
 * `rsautogui` aims to be a cross-platform GUI automation rust crate.
 * It lets you control the mouse and keyboard to automate interactions with other applications.
 * Currently, it's a wrapper of multiple rust crates to have pyautogui inspired syntax written in rust.

 
> **Warning**
> Not tested on Linux and MacOS
 
 ---

# Example Usage

## Mouse
```rust
use rsautogui::{mouse, mouse::MouseButton};

fn main() {
    let pos: (u16, u16) = mouse::position(); // Returns the current mouse coordinates.

    mouse::move_to(500, 500); // Moves mouse to x, y instantly.
    mouse::move_rel(500, 500); // Moves mouse to x, y relative to its position instantly.
    mouse::drag_to(500, 500); // Drags mouse to x, y instantly.
    mouse::drag_rel(500, 500); // Drags mouse to x, y relative to its position instantly.

    mouse::click(MouseButton::Left); // Performs a mouse click with the specified button.
    mouse::down(MouseButton::Left); // Performs a mouse down with the specified button.
    mouse::up(MouseButton::Left); // Performs a mouse up with the specified button.
    mouse::scroll('x', 10); // Scrolls x or y axis n times.
}
```
## Keyboard
```rust
use rsautogui::keyboard::{self, Key};

fn main() {
    keyboard::typewrite("lorem ipsum"); // Simulates typing the string provided.

    keyboard::key_down(Key::Layout('a')); // Presses specified key down.
    keyboard::key_up(Key::Layout('a')); // Releases specified key up.
    keyboard::key_tap(Key::Layout('a')); // Performs specified key up and down.
}
```
## Screen
```rust
use rsautogui::screen::{self, Rgba, ImageBuffer};

fn main() {
    let size: (u16, u16) = screen::size(); // Returns the width and height of primary screen.
    let on_screen: bool = screen::on_screen(1920, 1080); // Verifies if specified x & y coordinates are present on primary screen.

    let ss: ImageBuffer<Rgba<u8>, Vec<u8>> = screen::screenshot(0, 0, 1920, 1080); // Returns screenshot of the primary screen.
    screen::printscreen(&ss, "./src/assets/screenshot.jpg"); // Saves the provided screenshot to a path with the specified filename and extension.

    let loc: Option<(u16, u16)> = screen::locate_pixel(screen::Rgba([255, 255, 255, 255])); // Locates the first pixel color similar to the one specified and returns its coordinate.
    let loc: Vec<(u16, u16)> = screen::locate_all_pixel(screen::Rgba([255, 255, 255, 255])); // Locates all pixel colors similar to the one specified and returns their coordinates.
    let pixel: Rgba<u8> = screen::get_pixel(500, 500); // Get the pixel color on x, y coordinate.
}
```
---
You can read more documentation of this crate in [`docs.rs`](https://docs.rs/rsautogui/).
