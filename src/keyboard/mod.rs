//! # Keyboard Controls
//!
//! `keyboard` is a collection of functions used to control the keyboard.

pub use enigo::Key;
use enigo::{Enigo, KeyboardControllable};

/// Simulates typing the string provided.
pub fn typewrite(string: &str) {
    let mut enigo = Enigo::new();
    enigo.key_sequence_parse(string);
}

/// Presses specified key down.
pub fn key_down(key: Key) {
    let mut enigo = Enigo::new();
    enigo.key_down(key)
}

/// Releases specified key up.
pub fn key_up(key: Key) {
    let mut enigo = Enigo::new();
    enigo.key_up(key)
}

/// Performs specified key up and down.
pub fn key_tap(key: Key) {
    let mut enigo = Enigo::new();
    enigo.key_click(key)
}
