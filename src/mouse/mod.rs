//! # Mouse Controls
//!
//! `mouse` is a collection of functions used to control the mouse.

pub use enigo::MouseButton;
use enigo::{Enigo, MouseControllable};

/// Returns the current mouse coordinates.
pub fn position() -> (u16, u16) {
    let enigo = Enigo::new();
    let pos = enigo.mouse_location();
    return (pos.0.try_into().unwrap(), pos.1.try_into().unwrap());
}

/// Moves mouse to x, y instantly.
pub fn move_to(x: u16, y: u16) {
    let mut enigo = Enigo::new();
    enigo.mouse_move_to(x.try_into().unwrap(), y.try_into().unwrap())
}

// Moves mouse to x, y with the specified time in ms
// fn slow_move_to() {}

/// Moves mouse to x, y relative to its position instantly.
pub fn move_rel(x: u16, y: u16) {
    let mut enigo = Enigo::new();
    enigo.mouse_move_relative(x.try_into().unwrap(), y.try_into().unwrap())
}

// Moves mouse to x, y relative to its position with the specified time in ms
// fn slow_move_rel() {}

/// Drags mouse to x, y instantly.
pub fn drag_to(x: u16, y: u16) {
    let mut enigo = Enigo::new();
    enigo.mouse_down(MouseButton::Left);
    enigo.mouse_move_to(x.try_into().unwrap(), y.try_into().unwrap());
    enigo.mouse_up(MouseButton::Left)
}

// Drags mouse to x, y with the specified time in ms
// fn slow_drag_to() {}

/// Drags mouse to x, y relative to its position instantly.
pub fn drag_rel(x: u16, y: u16) {
    let mut enigo = Enigo::new();
    enigo.mouse_down(MouseButton::Left);
    enigo.mouse_move_relative(x.try_into().unwrap(), y.try_into().unwrap());
    enigo.mouse_up(MouseButton::Left)
}

// Drags mouse to x, y relative to its position with the specified time in ms
// fn slow_drag_rel() {}

/// Performs a mouse click with the specified button.
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
