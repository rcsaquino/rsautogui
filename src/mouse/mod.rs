//! # Mouse Controls
//!
//! `mouse` is a collection of functions used to control the mouse.

pub use enigo::MouseButton;
use enigo::{Enigo, MouseControllable};

/// Returns the current mouse coordinates.
pub fn position() -> (i32, i32) {
    Enigo::mouse_location()
}

/// Moves mouse to x, y instantly.
pub fn move_to(x: i32, y: i32) {
    let mut enigo = Enigo::new();
    enigo.mouse_move_to(x, y)
}

// Moves mouse to x, y with the specified time in ms
// fn slow_move_to() {}

/// Moves mouse to x, y relative to its position instantly.
pub fn move_rel(x: i32, y: i32) {
    let mut enigo = Enigo::new();
    enigo.mouse_move_relative(x, y)
}

// Moves mouse to x, y relative to its position with the specified time in ms
// fn slow_move_rel() {}

/// Drags mouse to x, y instantly.
pub fn drag_to(x: i32, y: i32) {
    let mut enigo = Enigo::new();
    enigo.mouse_down(MouseButton::Left);
    enigo.mouse_move_to(x, y);
    enigo.mouse_up(MouseButton::Left)
}

// Drags mouse to x, y with the specified time in ms
// fn slow_drag_to() {}

/// Drags mouse to x, y relative to its position instantly.
pub fn drag_rel(x: i32, y: i32) {
    let mut enigo = Enigo::new();
    enigo.mouse_down(MouseButton::Left);
    enigo.mouse_move_relative(x, y);
    enigo.mouse_up(MouseButton::Left)
}

// Drags mouse to x, y relative to its position with the specified time in ms
// fn slow_drag_rel() {}

/// Performs a mouse click.
pub fn click(button: MouseButton) {
    let mut enigo = Enigo::new();
    enigo.mouse_click(button);
}

/// Performs a mouse down with the specified button.
pub fn down(button: MouseButton) {
    let mut enigo = Enigo::new();
    enigo.mouse_down(button)
}

/// Performs a mouse up with the specified button.
pub fn up(button: MouseButton) {
    let mut enigo = Enigo::new();
    enigo.mouse_down(button)
}

/// Scrolls x or y axis n times.
pub fn scroll(axis: char, lines: i32) {
    let mut enigo = Enigo::new();
    match axis {
        'x' => enigo.mouse_scroll_x(lines),
        'y' => enigo.mouse_scroll_y(lines),
        _ => panic!("{} axis does not exist", axis),
    }
}
