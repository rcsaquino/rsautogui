//! # Screen Information
//!
//! `screen` is a collection of functions used to get screen related information.

pub use image::{DynamicImage, GenericImageView, Rgba};
pub use screenshots::{DisplayInfo, Screen};

/// Returns the width and height of primary screen.
pub fn size() -> (u16, u16) {
    let displays = DisplayInfo::all().expect("Unable to get displays");
    let primary = displays
        .iter()
        .find(|display| display.is_primary == true)
        .expect("Unable to find primary display");
    return (primary.width as u16, primary.height as u16);
}

/// Verifies if specified x & y coordinates are present on primary screen.
pub fn on_screen(x: u16, y: u16) -> bool {
    let display = size();
    return x <= display.0 && y <= display.1;
}

/// Returns screenshot of the primary screen.
pub fn screenshot(x: u16, y: u16, width: u16, height: u16) -> DynamicImage {
    if !on_screen(x + width, y + height) {
        panic!("One or more specified parameter is not within the screen size. Use screen::size() to check.")
    }
    let screen =
        Screen::from_point(x.into(), y.into()).expect("Cannot get screen from specified x and y");

    let capture = screen
        .capture_area(x.into(), y.into(), width.into(), height.into())
        .expect("Unable to screen capture.");

    let buffer = capture.buffer();

    return image::load_from_memory(buffer).unwrap();
}

/// Saves the provided screenshot to a path with the specified filename and extension.
pub fn printscreen(screenshot: DynamicImage, path: &str) {
    screenshot
        .save(path)
        .expect("Error saving file to specified path, filename, and/or extension.");
}

/// Locates the first pixel color similar to the one specified and returns its coordinate.
pub fn locate_pixel(pixel: Rgba<u8>) -> Option<(u16, u16)> {
    let display = size();
    let ss = screenshot(0, 0, display.0, display.1);
    for y in 0..display.1 {
        for x in 0..display.0 {
            if ss.get_pixel(x.into(), y.into()) == pixel {
                return Some((x, y));
            }
        }
    }
    return None;
}

/// Locates all pixel colors similar to the one specified and returns their coordinates.
pub fn locate_all_pixel(pixel: Rgba<u8>) -> Vec<(u16, u16)> {
    let display = size();
    let ss = screenshot(0, 0, display.0, display.1);
    let mut res: Vec<(u16, u16)> = Vec::new();
    for y in 0..display.1 {
        for x in 0..display.0 {
            if ss.get_pixel(x.into(), y.into()) == pixel {
                res.push((x, y))
            }
        }
    }
    return res;
}

/// Get the pixel color on x, y coordinate.
pub fn get_pixel(x: u16, y: u16) -> Rgba<u8> {
    let display = size();
    let ss = screenshot(0, 0, display.0, display.1);
    return ss.get_pixel(x.into(), y.into());
}

// Locates the first object similar to the one provided
// fn locate_img() {}

// Locates all objects similar to the image provided
// fn locate_all_img() {}

// Locates the first object similar to the one provided and returns its center x & y
// fn locate_img_center() {}
