//! # Mouse Controls
//!
//! `mouse` is a collection of functions used to control the mouse.

use std::time::Duration;
use winput::Mouse;

pub use winput::Button;

pub enum ScrollAxis {
    X,
    Y,
}

pub enum Speed {
    Fastest = 100,
    Faster = 250,
    Fast = 500,
    Normal = 1000,
    Slow = 1500,
    Slower = 2000,
    Slowest = 3000,
}

/// Returns the current mouse coordinates.
pub fn position() -> (u16, u16) {
    let (x, y) = Mouse::position().expect("Unable to locate mouse position.");
    return (x as u16, y as u16);
}

/// Moves mouse to x, y instantly.
pub fn move_to(x: u16, y: u16) {
    Mouse::set_position(x as i32, y as i32)
        .expect(&format!("Unable to move mouse position to ({}, {}).", x, y));
}

/// Moves mouse to x, y with the specified speed.
pub fn slow_move_to(x: u16, y: u16, speed: Speed) {
    let (mouse_x, mouse_y) = Mouse::position().expect("Unable to locate mouse position.");

    let delta_x = x as i32 - mouse_x;
    let delta_y = y as i32 - mouse_y;

    let delta_sum = (delta_x.pow(2) + delta_y.pow(2)) as f32;
    let distance = delta_sum.sqrt();

    // Calculate the movement vectors for both x and y directions
    let step_x = delta_x as f32 / distance as f32;
    let step_y = delta_y as f32 / distance as f32;

    // Move the mouse incrementally along the diagonal path to the target position
    let mut new_x = mouse_x as f32;
    let mut new_y = mouse_y as f32;
    let sleep_duration = Duration::from_micros(speed as u64);
    for _ in 0..distance as usize {
        new_x += step_x;
        new_y += step_y;
        Mouse::set_position(new_x as i32, new_y as i32)
            .expect(&format!("Unable to move mouse position to ({}, {}).", x, y));
        spin_sleep::sleep(sleep_duration)
    }
}

/// Moves mouse to x, y relative to its position instantly.
pub fn move_rel(x: i32, y: i32) {
    Mouse::move_relative(x, y);
}

/// Moves mouse to x, y relative to its position with the specified speed.
pub fn slow_move_rel(x: i32, y: i32, speed: Speed) {
    let delta_sum = (x.pow(2) + y.pow(2)) as f32;
    let distance = delta_sum.sqrt();

    // Calculate the movement vectors for both x and y directions
    let step_x = x as f32 / distance as f32;
    let step_y = y as f32 / distance as f32;

    // Move the mouse incrementally along the diagonal path to the target position
    let (mouse_x, mouse_y) = Mouse::position().expect("Unable to locate mouse position.");
    let mut new_x = mouse_x as f32;
    let mut new_y = mouse_y as f32;
    let sleep_duration = Duration::from_micros(speed as u64);
    for _ in 0..distance as usize {
        new_x += step_x;
        new_y += step_y;
        Mouse::set_position(new_x as i32, new_y as i32)
            .expect(&format!("Unable to move mouse position to ({}, {}).", x, y));
        spin_sleep::sleep(sleep_duration)
    }
}

/// Drags mouse to x, y instantly.
pub fn drag_to(x: u16, y: u16) {
    winput::press(Button::Left);
    Mouse::set_position(x as i32, y as i32)
        .expect(&format!("Unable to move mouse position to ({}, {}).", x, y));
    winput::release(Button::Left);
}

/// Drags mouse to x, y with the specified speed.
pub fn slow_drag_to(x: u16, y: u16, speed: Speed) {
    winput::press(Button::Left);
    slow_move_to(x, y, speed);
    winput::release(Button::Left);
}

/// Drags mouse to x, y relative to its position instantly.
pub fn drag_rel(x: i32, y: i32) {
    winput::press(Button::Left);
    move_rel(x, y);
    winput::release(Button::Left);
}

/// Drags mouse to x, y relative to its position with the specified speed.
pub fn slow_drag_rel(x: i32, y: i32, speed: Speed) {
    winput::press(Button::Left);
    slow_move_rel(x, y, speed);
    winput::release(Button::Left);
}

/// Performs a mouse click with the specified button.
pub fn click(button: Button) {
    winput::send(button);
}

/// Performs a mouse down with the specified button.
pub fn down(button: Button) {
    winput::press(button);
}

/// Performs a mouse up with the specified button.
pub fn up(button: Button) {
    winput::release(button);
}

/// Scrolls x or y axis n times.
pub fn scroll(axis: ScrollAxis, lines: i32) {
    match axis {
        ScrollAxis::X => Mouse::scrollh(lines as f32),
        ScrollAxis::Y => Mouse::scroll(lines as f32),
    }
}
