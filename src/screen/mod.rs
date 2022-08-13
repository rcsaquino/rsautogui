//! # Screen Information
//!
//! `screen` is a collection of functions used to get screen related information.

use image::{ImageBuffer, RgbaImage};
use scrap::{Capturer, Display};
use std::{thread, time};

pub use image::Rgba;

/// Returns the width and height of primary screen.
pub fn size() -> (u16, u16) {
    let display = Display::primary().expect("Can not get primary display.");
    return (display.width() as u16, display.height() as u16);
}

/// Verifies if specified x & y coordinates are present on primary screen.
pub fn on_screen(x: u16, y: u16) -> bool {
    let display = size();
    return x <= display.0 && y <= display.1;
}

/// Returns screenshot of the primary screen.
pub fn screenshot(width: u16, height: u16) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    if !on_screen(width, height) {
        panic!("Specified height and/or width are greater than screen size. Use screen::size() to check current screen size.")
    }
    
    let mut screen = Capturer::new(Display::primary().expect("Couldn't find display"))
        .expect("Couldn't capture screen");

    let mut ss: Vec<u8> = Vec::new();

    while ss.is_empty() {
        match screen.frame() {
            Ok(frame) => ss = frame.to_vec(),
            _ => thread::sleep(time::Duration::from_micros(1)),
        };
    }

    let mut img = RgbaImage::new(width.into(), height.into());

    let mut c: usize = 0;
    for y in 0..height {
        for x in 0..width {
            // ss is BGRA, so convert to RGBA
            img.put_pixel(
                x.into(),
                y.into(),
                Rgba([ss[c + 2], ss[c + 1], ss[c], ss[c + 3]]),
            );
            c += 4;
        }
    }
    return img;
}

/// Saves the provided screenshot to a path with the specified filename and extension.
pub fn printscreen(screenshot: &ImageBuffer<Rgba<u8>, Vec<u8>>, path: &str) {
    screenshot
        .save(path)
        .expect("Error saving file to specified path, filename, and/or extension.");
}

/// Locates the first pixel color similar to the one specified and returns its coordinate.
pub fn locate_pixel(pixel: Rgba<u8>) -> Option<(u16, u16)> {
    let display = size();
    let ss = screenshot(display.0, display.1);
    for y in 0..display.1 {
        for x in 0..display.0 {
            if *ss.get_pixel(x.into(), y.into()) == pixel {
                return Some((x, y));
            }
        }
    }
    return None;
}

/// Locates all pixel colors similar to the one specified and returns their coordinates.
pub fn locate_all_pixel(pixel: Rgba<u8>) -> Vec<(u16, u16)> {
    let display = size();
    let ss = screenshot(display.0, display.1);
    let mut res: Vec<(u16, u16)> = Vec::new();
    for y in 0..display.1 {
        for x in 0..display.0 {
            if *ss.get_pixel(x.into(), y.into()) == pixel {
                res.push((x, y))
            }
        }
    }
    return res;
}

/// Get the pixel color on x, y coordinate.
pub fn get_pixel(x: u16, y: u16) -> Rgba<u8> {
    let display = size();
    let ss = screenshot(display.0, display.1);
    return *ss.get_pixel(x.into(), y.into());
}

// Locates the first object similar to the one provided
// fn locate_img() {}

// Locates all objects similar to the image provided
// fn locate_all_img() {}

// Locates the first object similar to the one provided and returns its center x & y
// fn locate_img_center() {}

pub struct Screenshot {
    data: Vec<u8>,
    size: (u16, u16),
}

pub struct Bgra(u8, u8, u8, u8);

impl Screenshot {
    pub fn new() -> Screenshot {
        let mut screen = Capturer::new(Display::primary().expect("Couldn't find display"))
            .expect("Couldn't capture screen");
        let mut ss: Vec<u8> = Vec::new();

        while ss.is_empty() {
            match screen.frame() {
                Ok(frame) => ss = frame.to_vec(),
                _ => thread::sleep(time::Duration::from_micros(1)),
            };
        }

        return Screenshot {
            data: ss,
            size: (screen.width() as u16, screen.height() as u16),
        };
    }

    pub fn save(&self, path: &str) {
        let mut img = ImageBuffer::new(*&self.size.0 as u32, *&self.size.1 as u32);
        let mut i: usize = 0;
        for pixel in img.pixels_mut() {
            let b = *&self.data[i];
            let g = *&self.data[i + 1];
            let r = *&self.data[i + 2];
            let a = *&self.data[i + 3];
            *pixel = image::Rgba([r, g, b, a]);
            i += 4;
        }

        img.save(path).unwrap();
    }
}
