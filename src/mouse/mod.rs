//! # Mouse controls
//!
//! `mouse` is a collection of functions used to control the mouse.
//!
//! Works on Windows, Linux (X11) and macOS with no dependencies.

use std::time::{Duration, Instant};

use crate::platform as plat;

/// A mouse button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Button {
    Left,
    Right,
    Middle,
    X1,
    X2,
}

/// The axis for [`scroll`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScrollAxis {
    X,
    Y,
}

/// Speed for the `slow_*` movement functions (microseconds per step).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Speed {
    Fastest = 100,
    Faster = 250,
    Fast = 500,
    Normal = 1000,
    Slow = 1500,
    Slower = 2000,
    Slowest = 3000,
}

fn sleep_micros(micros: u64) {
    if micros >= 1_000 {
        std::thread::sleep(Duration::from_micros(micros));
    } else if micros > 0 {
        // Sub-millisecond sleeps via spin (std::thread::sleep granularity is
        // ~1ms, and the old spin_sleep dependency is gone).
        let deadline = Instant::now() + Duration::from_micros(micros);
        while Instant::now() < deadline {
            std::hint::spin_loop();
        }
    }
}

/// Returns the current mouse coordinates.
pub fn position() -> (u16, u16) {
    plat::mouse_position()
}

/// Moves mouse to x, y instantly.
pub fn move_to(x: u16, y: u16) {
    plat::mouse_set_position(x, y);
}

/// Moves mouse to x, y with the specified speed.
pub fn slow_move_to(x: u16, y: u16, speed: Speed) {
    let (mouse_x, mouse_y) = plat::mouse_position();

    let delta_x = x as i32 - mouse_x as i32;
    let delta_y = y as i32 - mouse_y as i32;

    let distance = ((delta_x.pow(2) + delta_y.pow(2)) as f32).sqrt();
    if distance < 1.0 {
        plat::mouse_set_position(x, y);
        return;
    }

    let step_x = delta_x as f32 / distance;
    let step_y = delta_y as f32 / distance;

    let mut new_x = mouse_x as f32;
    let mut new_y = mouse_y as f32;
    for _ in 0..distance as usize {
        new_x += step_x;
        new_y += step_y;
        plat::mouse_set_position(new_x as u16, new_y as u16);
        sleep_micros(speed as u64);
    }
    plat::mouse_set_position(x, y);
}

/// Moves mouse to x, y relative to its position instantly.
pub fn move_rel(x: i32, y: i32) {
    plat::mouse_move_relative(x, y);
}

/// Moves mouse to x, y relative to its position with the specified speed.
pub fn slow_move_rel(x: i32, y: i32, speed: Speed) {
    let distance = ((x.pow(2) + y.pow(2)) as f32).sqrt();
    if distance < 1.0 {
        return;
    }
    let step_x = x as f32 / distance;
    let step_y = y as f32 / distance;

    let (mouse_x, mouse_y) = plat::mouse_position();
    let mut new_x = mouse_x as f32;
    let mut new_y = mouse_y as f32;
    for _ in 0..distance as usize {
        new_x += step_x;
        new_y += step_y;
        plat::mouse_set_position(new_x as u16, new_y as u16);
        sleep_micros(speed as u64);
    }
    plat::mouse_move_relative(
        x - (new_x as i32 - mouse_x as i32),
        y - (new_y as i32 - mouse_y as i32),
    );
}

/// Drags mouse to x, y instantly (holds the left button while moving).
pub fn drag_to(x: u16, y: u16) {
    plat::mouse_press(Button::Left);
    plat::mouse_set_position(x, y);
    plat::mouse_release(Button::Left);
}

/// Drags mouse to x, y with the specified speed.
pub fn slow_drag_to(x: u16, y: u16, speed: Speed) {
    plat::mouse_press(Button::Left);
    slow_move_to(x, y, speed);
    plat::mouse_release(Button::Left);
}

/// Drags mouse to x, y relative to its position instantly.
pub fn drag_rel(x: i32, y: i32) {
    plat::mouse_press(Button::Left);
    plat::mouse_move_relative(x, y);
    plat::mouse_release(Button::Left);
}

/// Drags mouse to x, y relative to its position with the specified speed.
pub fn slow_drag_rel(x: i32, y: i32, speed: Speed) {
    plat::mouse_press(Button::Left);
    slow_move_rel(x, y, speed);
    plat::mouse_release(Button::Left);
}

/// Performs a mouse click with the specified button.
pub fn click(button: Button) {
    plat::mouse_press(button);
    plat::mouse_release(button);
}

/// Performs a mouse down with the specified button.
pub fn down(button: Button) {
    plat::mouse_press(button);
}

/// Performs a mouse up with the specified button.
pub fn up(button: Button) {
    plat::mouse_release(button);
}

/// Scrolls the x or y axis `lines` times.
///
/// Positive values scroll up (Y) or right (X); negative values scroll down
/// (Y) or left (X).
pub fn scroll(axis: ScrollAxis, lines: i32) {
    match axis {
        ScrollAxis::X => plat::mouse_scroll_horizontal(lines),
        ScrollAxis::Y => plat::mouse_scroll_vertical(lines),
    }
}
