//! # Screen information
//!
//! `screen` is a collection of functions used to get screen related
//! information and screenshots.
//!
//! Screenshots are returned as [`Image`] (RGBA8) from the dependency-free
//! [`crate::image`] module. [`DynamicImage`] is kept as an alias so existing
//! code mentioning `screen::DynamicImage` keeps compiling.

use crate::platform as plat;

pub use crate::image::{Image, Rgba};

/// Backwards-compatible alias for [`Image`].
pub type DynamicImage = Image;

/// Returns the width and height of the primary screen.
pub fn size() -> (u16, u16) {
    plat::screen_size()
}

/// Verifies if specified x & y coordinates are present on the primary screen.
pub fn on_screen(x: u16, y: u16) -> bool {
    let (w, h) = plat::screen_size();
    x <= w && y <= h
}

/// Returns a screenshot of the given region of the primary screen.
///
/// # Panics
/// Panics if the region is not within the screen; use [`size`] to check.
pub fn screenshot(x: u16, y: u16, width: u16, height: u16) -> DynamicImage {
    let (w, h) = plat::screen_size();
    if x.saturating_add(width) > w || y.saturating_add(height) > h {
        panic!(
            "One or more specified parameters are not within the screen size ({}, {}). Use screen::size() to check.",
            w, h
        );
    }
    plat::screenshot_capture(x, y, width, height)
}

/// Saves the provided screenshot to a path.
///
/// The format is inferred from the extension: `.png`, `.bmp` or `.ppm`.
/// (JPEG is not supported by the dependency-free encoder; save as PNG.)
pub fn printscreen(screenshot: DynamicImage, path: &str) {
    screenshot
        .save(path)
        .expect("Error saving file to specified path, filename, and/or extension.");
}

/// Locates the first pixel color equal to the one specified and returns its
/// coordinate.
pub fn locate_pixel(pixel: Rgba) -> Option<(u16, u16)> {
    let (w, h) = plat::screen_size();
    let ss = plat::screenshot_capture(0, 0, w, h);
    for y in 0..h as u32 {
        for x in 0..w as u32 {
            if ss.get_pixel(x, y) == pixel {
                return Some((x as u16, y as u16));
            }
        }
    }
    None
}

/// Locates all pixel colors equal to the one specified and returns their
/// coordinates.
pub fn locate_all_pixel(pixel: Rgba) -> Vec<(u16, u16)> {
    let (w, h) = plat::screen_size();
    let ss = plat::screenshot_capture(0, 0, w, h);
    let mut res: Vec<(u16, u16)> = Vec::new();
    for y in 0..h as u32 {
        for x in 0..w as u32 {
            if ss.get_pixel(x, y) == pixel {
                res.push((x as u16, y as u16));
            }
        }
    }
    res
}

/// Gets the pixel color on x, y coordinate.
pub fn get_pixel(x: u16, y: u16) -> Rgba {
    // Capture just 1x1 — much cheaper than a full-screen grab.
    let ss = plat::screenshot_capture(x, y, 1, 1);
    ss.get_pixel(0, 0)
}

// Locates the first object similar to the one provided
// fn locate_img() {}

// Locates all objects similar to the image provided
// fn locate_all_img() {}

// Locates the first object similar to the one provided and returns its center x & y
// fn locate_img_center() {}
