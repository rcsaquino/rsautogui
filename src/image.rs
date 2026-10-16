//! # Minimal image type
//!
//! [`Image`] is a tiny RGBA8 image used for screenshots. It intentionally
//! mirrors the subset of `image::DynamicImage` / `image::Rgba` that
//! `rsautogui` needs, without pulling in any dependency.
//!
//! Saving supports:
//! * `.bmp` — 32-bit uncompressed BMP (BGRA, top-down).
//! * `.png` — 32-bit RGBA PNG (implemented with stored / uncompressed
//!   DEFLATE blocks, so no compression library is required).
//! * `.ppm` — binary P6 PPM (RGB, alpha is dropped).
//!
//! JPEG is intentionally *not* supported: a conforming encoder needs DCT +
//! Huffman coding. Use PNG or BMP instead.

pub(crate) mod png;

use std::io;
use std::ops::Index;
use std::path::Path;

/// An RGBA pixel. Layout-compatible with `image::Rgba([u8; 4])` for easy
/// migration: construct with `Rgba([r, g, b, a])` and read with `.0`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Rgba(pub [u8; 4]);

impl Rgba {
    /// Red channel.
    #[inline]
    pub fn r(self) -> u8 {
        self.0[0]
    }
    /// Green channel.
    #[inline]
    pub fn g(self) -> u8 {
        self.0[1]
    }
    /// Blue channel.
    #[inline]
    pub fn b(self) -> u8 {
        self.0[2]
    }
    /// Alpha channel.
    #[inline]
    pub fn a(self) -> u8 {
        self.0[3]
    }
}

impl From<[u8; 4]> for Rgba {
    #[inline]
    fn from(v: [u8; 4]) -> Self {
        Rgba(v)
    }
}

impl From<Rgba> for [u8; 4] {
    #[inline]
    fn from(v: Rgba) -> Self {
        v.0
    }
}

impl Index<usize> for Rgba {
    type Output = u8;
    #[inline]
    fn index(&self, index: usize) -> &u8 {
        &self.0[index]
    }
}

/// An 8-bit RGBA image, row-major, top-down.
///
/// `data.len() == width * height * 4`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Image {
    width: u32,
    height: u32,
    data: Vec<u8>,
}

impl Image {
    /// Creates an image from raw RGBA bytes.
    ///
    /// # Panics
    /// Panics if `data.len() != width * height * 4`.
    pub fn from_rgba(width: u32, height: u32, data: Vec<u8>) -> Self {
        assert_eq!(
            data.len(),
            width as usize * height as usize * 4,
            "Image data length does not match dimensions"
        );
        Self {
            width,
            height,
            data,
        }
    }

    /// Width in pixels.
    #[inline]
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Height in pixels.
    #[inline]
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Raw RGBA bytes, row-major, top-down.
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    /// Raw RGBA bytes, mutable.
    #[inline]
    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }

    /// Consumes the image and returns the raw RGBA buffer.
    #[inline]
    pub fn into_bytes(self) -> Vec<u8> {
        self.data
    }

    /// Decodes a PNG file (8-bit, color types 0/2/4/6) into an image.
    pub fn from_png_bytes(bytes: &[u8]) -> io::Result<Self> {
        png::decode_png(bytes)
    }

    /// Downscales by integer `factor` using box averaging (for HiDPI
    /// screenshot buffers, which are `factor` denser than logical pixels).
    ///
    /// # Panics
    /// Panics if `factor == 0` or the dimensions are not divisible by it.
    pub fn downscale(&self, factor: u32) -> Self {
        assert!(factor >= 1, "downscale factor must be >= 1");
        if factor == 1 {
            return self.clone();
        }
        let f = factor as usize;
        assert!(
            self.width as usize % f == 0 && self.height as usize % f == 0,
            "image {}x{} not divisible by downscale factor {}",
            self.width,
            self.height,
            factor
        );
        let (nw, nh) = (self.width / factor, self.height / factor);
        let mut data = vec![0u8; nw as usize * nh as usize * 4];
        let sw = self.width as usize;
        let n = (f * f) as u32;
        for dy in 0..nh as usize {
            for dx in 0..nw as usize {
                let mut acc = [0u32; 4];
                for sy in 0..f {
                    for sx in 0..f {
                        let i = ((dy * f + sy) * sw + dx * f + sx) * 4;
                        for (a, b) in acc.iter_mut().zip(&self.data[i..i + 4]) {
                            *a += *b as u32;
                        }
                    }
                }
                // Round to nearest.
                let o = (dy * nw as usize + dx) * 4;
                for (d, a) in data[o..o + 4].iter_mut().zip(&acc) {
                    *d = ((a + n / 2) / n).min(255) as u8;
                }
            }
        }
        Self {
            width: nw,
            height: nh,
            data,
        }
    }

    /// Returns the sub-region `(x, y, width, height)` as a new image.
    ///
    /// # Panics
    /// Panics if the region exceeds the image bounds.
    pub fn crop(&self, x: u32, y: u32, width: u32, height: u32) -> Self {
        assert!(
            x.saturating_add(width) <= self.width && y.saturating_add(height) <= self.height,
            "crop region ({x}, {y}, {width}, {height}) exceeds {}x{} image",
            self.width,
            self.height
        );
        let mut data = vec![0u8; width as usize * height as usize * 4];
        let sw = self.width as usize;
        for row in 0..height as usize {
            let src = ((y as usize + row) * sw + x as usize) * 4;
            let dst = row * width as usize * 4;
            data[dst..dst + width as usize * 4]
                .copy_from_slice(&self.data[src..src + width as usize * 4]);
        }
        Self {
            width,
            height,
            data,
        }
    }

    /// Returns the pixel at `(x, y)`.
    ///
    /// # Panics
    /// Panics if out of bounds.
    pub fn get_pixel(&self, x: u32, y: u32) -> Rgba {
        assert!(
            x < self.width && y < self.height,
            "pixel ({}, {}) out of bounds for {}x{} image",
            x,
            y,
            self.width,
            self.height
        );
        let i = (y as usize * self.width as usize + x as usize) * 4;
        Rgba([self.data[i], self.data[i + 1], self.data[i + 2], self.data[i + 3]])
    }

    /// Sets the pixel at `(x, y)`.
    ///
    /// # Panics
    /// Panics if out of bounds.
    pub fn put_pixel(&mut self, x: u32, y: u32, px: Rgba) {
        assert!(
            x < self.width && y < self.height,
            "pixel ({}, {}) out of bounds for {}x{} image",
            x,
            y,
            self.width,
            self.height
        );
        let i = (y as usize * self.width as usize + x as usize) * 4;
        self.data[i..i + 4].copy_from_slice(&px.0);
    }

    /// Saves to `path`. The format is inferred from the extension:
    /// `.png`, `.bmp`, `.ppm`. Anything else (including `.jpg`/`.jpeg`)
    /// returns an error.
    pub fn save(&self, path: impl AsRef<Path>) -> io::Result<()> {
        let path = path.as_ref();
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let bytes: Vec<u8> = match ext.as_str() {
            "png" => encode_png(self.width, self.height, &self.data),
            "bmp" => encode_bmp(self.width, self.height, &self.data),
            "ppm" => encode_ppm(self.width, self.height, &self.data),
            "jpg" | "jpeg" => {
                return Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    "rsautogui is dependency-free and does not encode JPEG; save as .png or .bmp instead",
                ))
            }
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!(
                        "unsupported image extension '.{}'; use .png, .bmp or .ppm",
                        ext
                    ),
                ))
            }
        };
        std::fs::write(path, bytes)
    }
}

// ---------------------------------------------------------------------------
// Encoders (private)
// ---------------------------------------------------------------------------

pub(crate) fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            if crc & 1 == 1 {
                crc = (crc >> 1) ^ 0xEDB8_8320;
            } else {
                crc >>= 1;
            }
        }
    }
    crc ^ 0xFFFF_FFFF
}

pub(crate) fn adler32(data: &[u8]) -> u32 {
    const MOD: u32 = 65521;
    let mut a: u32 = 1;
    let mut b: u32 = 0;
    for &byte in data {
        a = (a + byte as u32) % MOD;
        b = (b + a) % MOD;
    }
    (b << 16) | a
}

fn write_png_chunk(out: &mut Vec<u8>, ty: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(ty);
    out.extend_from_slice(data);
    let mut crc_input = Vec::with_capacity(4 + data.len());
    crc_input.extend_from_slice(ty);
    crc_input.extend_from_slice(data);
    out.extend_from_slice(&crc32(&crc_input).to_be_bytes());
}

fn encode_png(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&[137, 80, 78, 71, 13, 10, 26, 10]);

    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.push(8); // bit depth
    ihdr.push(6); // color type: RGBA
    ihdr.push(0); // compression
    ihdr.push(0); // filter
    ihdr.push(0); // interlace
    write_png_chunk(&mut out, b"IHDR", &ihdr);

    // Filtered scanlines (filter type 0 = None).
    let stride = width as usize * 4;
    let mut raw = Vec::with_capacity(height as usize * (stride + 1));
    for y in 0..height as usize {
        raw.push(0);
        let s = y * stride;
        raw.extend_from_slice(&rgba[s..s + stride]);
    }

    // zlib stream using only stored (uncompressed) DEFLATE blocks.
    let mut z = Vec::with_capacity(raw.len() + 16);
    z.push(0x78);
    z.push(0x01);
    if raw.is_empty() {
        z.push(1);
        z.extend_from_slice(&0u16.to_le_bytes());
        z.extend_from_slice(&0xFFFFu16.to_le_bytes());
    } else {
        let mut i = 0;
        while i < raw.len() {
            let end = (i + 65535).min(raw.len());
            let last = end == raw.len();
            z.push(if last { 1 } else { 0 });
            let len = (end - i) as u16;
            z.extend_from_slice(&len.to_le_bytes());
            z.extend_from_slice(&(!len).to_le_bytes());
            z.extend_from_slice(&raw[i..end]);
            i = end;
        }
    }
    z.extend_from_slice(&adler32(&raw).to_be_bytes());
    write_png_chunk(&mut out, b"IDAT", &z);
    write_png_chunk(&mut out, b"IEND", &[]);
    out
}

fn encode_bmp(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    let pixel_bytes = width as usize * height as usize * 4;
    let file_size = 14u32 + 40 + pixel_bytes as u32;
    let mut out = Vec::with_capacity(file_size as usize);
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&file_size.to_le_bytes());
    out.extend_from_slice(&[0, 0, 0, 0]);
    out.extend_from_slice(&54u32.to_le_bytes());
    // DIB header (BITMAPINFOHEADER).
    out.extend_from_slice(&40u32.to_le_bytes());
    out.extend_from_slice(&(width as i32).to_le_bytes());
    // Negative height => top-down rows, matching our buffer order.
    out.extend_from_slice(&(-(height as i32)).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&32u16.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // BI_RGB
    out.extend_from_slice(&(pixel_bytes as u32).to_le_bytes());
    out.extend_from_slice(&2835u32.to_le_bytes());
    out.extend_from_slice(&2835u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    // RGBA -> BGRA.
    for px in rgba.chunks_exact(4) {
        out.push(px[2]);
        out.push(px[1]);
        out.push(px[0]);
        out.push(px[3]);
    }
    out
}

fn encode_ppm(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity((width as usize) * (height as usize) * 3 + 32);
    out.extend_from_slice(format!("P6\n{} {}\n255\n", width, height).as_bytes());
    for px in rgba.chunks_exact(4) {
        out.push(px[0]);
        out.push(px[1]);
        out.push(px[2]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn png_roundtrip_signature() {
        let img = Image::from_rgba(2, 1, vec![255, 0, 0, 255, 0, 255, 0, 255]);
        let png = encode_png(img.width(), img.height(), img.as_bytes());
        assert_eq!(&png[0..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
        assert!(png.len() > 8);
    }

    #[test]
    fn bmp_header() {
        let img = Image::from_rgba(1, 1, vec![1, 2, 3, 255]);
        let bmp = encode_bmp(1, 1, img.as_bytes());
        assert_eq!(&bmp[0..2], b"BM");
        // BGRA order.
        assert_eq!(&bmp[54..58], &[3, 2, 1, 255]);
    }

    #[test]
    fn downscale_box() {
        // 4x4 -> 2x2, factor 2, solid quadrants.
        let mut data = vec![0u8; 4 * 4 * 4];
        for y in 0..4usize {
            for x in 0..4usize {
                let i = (y * 4 + x) * 4;
                let v = if x < 2 { 100 } else { 200 };
                data[i..i + 4].copy_from_slice(&[v, v, v, 255]);
            }
        }
        let img = Image::from_rgba(4, 4, data);
        let small = img.downscale(2);
        assert_eq!((small.width(), small.height()), (2, 2));
        assert_eq!(small.get_pixel(0, 0), Rgba([100, 100, 100, 255]));
        assert_eq!(small.get_pixel(1, 0), Rgba([200, 200, 200, 255]));
        assert_eq!(img.downscale(1).as_bytes(), img.as_bytes());
    }

    #[test]
    fn pixel_access() {
        let mut img = Image::from_rgba(2, 2, vec![0; 16]);
        img.put_pixel(1, 0, Rgba([9, 8, 7, 6]));
        assert_eq!(img.get_pixel(1, 0), Rgba([9, 8, 7, 6]));
    }
}
