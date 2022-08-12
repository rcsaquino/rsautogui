use rsautogui::screen::{printscreen, screenshot, Screenshot};

fn main() {
    let time = std::time::Instant::now();
    let ss = Screenshot::new();
    ss.save("test.png");
    // let ss = screenshot(1920, 1080);
    // printscreen(&ss, "test.png");
    println!("{:?}", time.elapsed())
}
