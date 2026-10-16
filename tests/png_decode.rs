//! PNG decoder tests against fixtures produced by a real encoder (zlib
//! level 9): stored, fixed-Huffman and dynamic-Huffman blocks, all five
//! filter types, and color types 0/2/4/6.

use rsautogui::image::{Image, Rgba};

fn fixture_dir() -> String {
    format!("{}/tests/fixtures", env!("CARGO_MANIFEST_DIR"))
}

fn check(name: &str, w: u32, h: u32) {
    let dir = fixture_dir();
    let png = std::fs::read(format!("{dir}/{name}.png"))
        .unwrap_or_else(|e| panic!("missing fixture {name}.png: {e}"));
    let expected = std::fs::read(format!("{dir}/{name}.rgba"))
        .unwrap_or_else(|e| panic!("missing fixture {name}.rgba: {e}"));
    let img = Image::from_png_bytes(&png)
        .unwrap_or_else(|e| panic!("failed decoding {name}.png: {e}"));
    assert_eq!((img.width(), img.height()), (w, h), "{name}: dimensions");
    assert_eq!(
        img.as_bytes(),
        expected.as_slice(),
        "{name}: pixel mismatch"
    );
}

#[test]
fn decode_fixed_huffman_gray() {
    check("flat_gray_8x8", 8, 8);
}

#[test]
fn decode_stored_rgb() {
    check("grad_rgb_35x17", 35, 17);
}

#[test]
fn decode_dynamic_rgba() {
    check("grad_rgba_16x16", 16, 16);
}

#[test]
fn decode_stored_large_rgb() {
    check("grad_rgb_64x64", 64, 64);
}

#[test]
fn decode_dynamic_lz_bands() {
    // Highly compressible: long matches + far distances + dynamic Huffman.
    check("bands_128x128", 128, 128);
}

#[test]
fn decode_dynamic_gray_alpha() {
    check("graya_16x16", 16, 16);
}

#[test]
fn encoder_decoder_roundtrip() {
    // Our own encoder (stored blocks) -> our decoder.
    let mut img = Image::from_rgba(7, 5, vec![0u8; 7 * 5 * 4]);
    for y in 0..5 {
        for x in 0..7 {
            img.put_pixel(x, y, Rgba([(x * 37) as u8, (y * 53) as u8, 200, 255]));
        }
    }
    let path = std::env::temp_dir().join("rsautogui-roundtrip-test.png");
    img.save(&path).expect("save png");
    let bytes = std::fs::read(&path).expect("read png");
    let back = Image::from_png_bytes(&bytes).expect("decode own png");
    assert_eq!((back.width(), back.height()), (7, 5));
    assert_eq!(back.as_bytes(), img.as_bytes());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rejects_garbage() {
    assert!(Image::from_png_bytes(b"definitely not a png").is_err());
    assert!(Image::from_png_bytes(&[137, 80, 78, 71, 13, 10, 26, 10]).is_err());
}
