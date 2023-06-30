# rsautogui [Image-Recognition-Branch]
This branch is used to hold the image-recognition until successfully merged into rsautogui.

Created and implemented the first image recognition for rsautogui. It can search the entire screen a over 3x faster than pyautogui and does not require OpenCV for options like Confidence. I've also added an option called Tolerance that allows for leniency with pixel colors that are close to the original image's. Written in pure Rust.
```rust
locate_img_center(img: &DynamicImage, region: Option<(u16, u16, u16, u16)>, min_confidence: Option<f32>, tolerance: Option<u8>) -> Option<(u32, u32, f32)>)
```
```rust
locate_img(img: &DynamicImage, region: Option<(u16, u16, u16, u16)>, min_confidence: Option<f32>, tolerance: Option<u8>) 
```
**img**: required borrowed DynamicImage

**region**: requires tuple BoundingBox (x, y, width, height) (Default Entire Screen)

**min_confidence**: 0.1 - 1.0, percentage of how many of the pixels need to match (Default 0.95)

**tolerance**: 0 - 255, range of pixels to accept from image's pixels. So if an image has a pixel of 234, 52, 245 with a tolerance of 10, then the locator will accept values ranging from 224, 42, 235 - 244, 62, 255. (Default 5)


All of these requires (except img) are optional and require either a **Some()** or **None**

Examples:
```rust
fn main() {
    let img = image::open("images.png").expect("Unable to locate file.");    
    match locate_img_center(&img, None, Some(0.9), Some(10)) {
        Some((x, y, confidence)) => {
            println!("Image center found at {}, {} with confidence {}", x, y, confidence);
            move_to(x.try_into().unwrap(), y.try_into().unwrap())
        },
        None => println!("Image not found"),
    }
}
```
```rust
fn main() {
    let img = image::open("images.png").expect("Unable to locate file.");
    match locate_image(&img, None, None, None) {
        Some((x, y, img_width, img_height, _confidence)) => {
            println!("x: {}, y: {}, width: {}, height: {}",x, y, img_width, img_height)
        },
        None => println!("Image not found")
    }
}
```
Early version of this had incredible speed during development.  But as new features were added and simplicity introduced, started to slow down itself.

Benchmark
--------
**Python**: 150ms
**Rust**: 53ms
(Both finding the Rust icon with a size of 225x225px on a 1920x1080 screen)

