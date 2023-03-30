//! # Keyboard Controls
//!
//! `keyboard` is a collection of functions used to control the keyboard.

use std::collections::HashMap;
pub use enigo::Key;
use enigo::{Enigo, KeyboardControllable};

/// Simulates typing the string provided.
pub fn typewrite(string: &str) {
    let mut unique_keys: HashMap<String, String> = HashMap::new();
    unique_keys.insert("~".to_string(), "`".to_string());
    unique_keys.insert("!".to_string(), "1".to_string());
    unique_keys.insert("@".to_string(), "2".to_string());
    unique_keys.insert("#".to_string(), "3".to_string());
    unique_keys.insert("$".to_string(), "4".to_string());
    unique_keys.insert("%".to_string(), "5".to_string());
    unique_keys.insert("^".to_string(), "6".to_string());
    unique_keys.insert("&".to_string(), "7".to_string());
    unique_keys.insert("*".to_string(), "8".to_string());
    unique_keys.insert("(".to_string(), "9".to_string());
    unique_keys.insert(")".to_string(), "0".to_string());
    unique_keys.insert("_".to_string(), "-".to_string());
    unique_keys.insert("+".to_string(), "=".to_string());
    unique_keys.insert("{".to_string(), "[".to_string());
    unique_keys.insert("}".to_string(), "]".to_string());
    unique_keys.insert("|".to_string(), "\\".to_string());
    unique_keys.insert(":".to_string(), ";".to_string());
    unique_keys.insert("\"".to_string(), "'".to_string());
    unique_keys.insert("<".to_string(), ",".to_string());
    unique_keys.insert(">".to_string(), ".".to_string());
    unique_keys.insert("?".to_string(), "/".to_string());
    


    let mut sequence = String::from("");
    for letter in string.chars() {

        if letter.is_uppercase() {
            let prep_letter = "{+SHIFT}".to_string() + &letter.to_lowercase().to_string() + "{-SHIFT}";
            sequence.push_str(&prep_letter);
            
        } else if let Some(value) = unique_keys.get(&letter.to_string()) {
            let prep_letter = "{+SHIFT}".to_string() + value + "{-SHIFT}";
            sequence.push_str(&prep_letter);
        }
         else {
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
