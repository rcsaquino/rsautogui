//! # Keyboard Controls
//!
//! `keyboard` is a collection of functions used to control the keyboard.

pub use enigo::Key;
use enigo::{Enigo, KeyboardControllable};
use std::collections::HashMap;

/// Simulates typing the string provided.
pub fn typewrite(string: &str) {
    let mut unique_keys: HashMap<char, char> = HashMap::new();
    unique_keys.insert('~', '`');
    unique_keys.insert('!', '1');
    unique_keys.insert('@', '2');
    unique_keys.insert('#', '3');
    unique_keys.insert('$', '4');
    unique_keys.insert('%', '5');
    unique_keys.insert('^', '6');
    unique_keys.insert('&', '7');
    unique_keys.insert('*', '8');
    unique_keys.insert('(', '9');
    unique_keys.insert(')', '0');
    unique_keys.insert('_', '-');
    unique_keys.insert('+', '=');
    unique_keys.insert('{', '[');
    unique_keys.insert('}', ']');
    unique_keys.insert('|', '\\');
    unique_keys.insert(':', ';');
    unique_keys.insert('\'', '\'');
    unique_keys.insert('<', ',');
    unique_keys.insert('>', '.');
    unique_keys.insert('?', '/');

    let mut sequence = String::new();
    for letter in string.chars() {
        if letter.is_uppercase() {
            let prep_letter =
                "{+SHIFT}".to_string() + &letter.to_lowercase().to_string() + "{-SHIFT}";
            sequence.push_str(&prep_letter);
        } else if let Some(val) = unique_keys.get(&letter) {
            let prep_letter = "{+SHIFT}".to_string() + &val.to_string() + "{-SHIFT}";
            sequence.push_str(&prep_letter);
        } else {
            sequence.push(letter)
        }
    }
    let mut enigo = Enigo::new();
    enigo.key_sequence_parse(&sequence);
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
