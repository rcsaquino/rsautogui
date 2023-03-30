//! # rsautogui
//!
//! `rsautogui` aims to be a cross-platform GUI automation rust crate.
//! It lets you control the mouse and keyboard to automate interactions with other applications.
//! Currently, it's a wrapper of multiple rust crates to have pyautogui inspired syntax in rust.
//!
//! This crate takes heavy inspiration from [PyAutoGUI.](https://github.com/asweigart/pyautogui)

pub mod keyboard;
pub mod mouse;
pub mod screen;
