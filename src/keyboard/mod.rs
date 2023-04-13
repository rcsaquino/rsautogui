//! # Keyboard Controls
//!
//! `keyboard` is a collection of functions used to control the keyboard.

use winput::Keylike;

pub use winput::Vk;

#[deprecated(since="0.2.0", note="Please use virtual key enum `Vk` instead.")]
pub use winput::Vk as Key;

/// Simulates typing the string provided.
pub fn typewrite(string: &str) {
    winput::send_str(string);
}

/// Presses specified key down.
pub fn key_down<K: Keylike>(key: K) {
    winput::press(key);
}

/// Releases specified key up.
pub fn key_up<K: Keylike>(key: K) {
    winput::release(key);
}

/// Performs specified key up and down.
pub fn key_tap<K: Keylike>(key: K) {
    winput::send(key)
}
