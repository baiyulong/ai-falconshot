use capture_core::CaptureBackend;
use platform_windows::capture::WindowsCaptureBackend;

fn main() {
    println!("=== FalconShot Capture Engine Test ===\n");

    let backend = WindowsCaptureBackend::new().expect("Failed to create capture backend");

    // 1. Enumerate monitors
    println!("--- Monitors ---");
    let monitors = backend
        .enumerate_monitors()
        .expect("Failed to enumerate monitors");
    for m in &monitors {
        println!(
            "  [{}] {} | {}x{} @ ({},{}) | scale: {:.2} | primary: {}",
            m.index,
            m.name,
            m.bounds.width,
            m.bounds.height,
            m.bounds.x,
            m.bounds.y,
            m.scale_factor,
            m.is_primary
        );
        println!(
            "       work_area: {}x{} @ ({},{})",
            m.work_area.width, m.work_area.height, m.work_area.x, m.work_area.y
        );
    }
    println!();

    // 2. Enumerate windows (first 10)
    println!("--- Visible Windows (first 10) ---");
    let windows = backend
        .enumerate_windows()
        .expect("Failed to enumerate windows");
    for w in windows.iter().take(10) {
        println!(
            "  [0x{:X}] \"{}\" ({}) | {}x{} @ ({},{})",
            w.id, w.title, w.class_name, w.bounds.width, w.bounds.height, w.bounds.x, w.bounds.y
        );
    }
    println!("  ... total: {} windows\n", windows.len());

    // 3. Capture a 400x300 region from top-left of primary monitor
    println!("--- Capture Region (400x300) ---");
    let options = capture_core::CaptureOptions {
        region: Some(capture_core::Rect::new(0, 0, 400, 300)),
        ..Default::default()
    };
    let frame = backend
        .capture_region(&options)
        .expect("Failed to capture region");
    println!(
        "  Captured: {}x{} | timestamp: {} | monitor: {}",
        frame.image.width(),
        frame.image.height(),
        frame.timestamp_ms,
        frame.monitor.name
    );

    let output_path = "test_capture_region.png";
    frame.image.save(output_path).expect("Failed to save image");
    println!("  Saved to: {}\n", output_path);

    // 4. Capture fullscreen
    println!("--- Capture Fullscreen ---");
    let frame = backend
        .capture_fullscreen()
        .expect("Failed to capture fullscreen");
    println!(
        "  Captured: {}x{} | timestamp: {}",
        frame.image.width(),
        frame.image.height(),
        frame.timestamp_ms
    );

    let output_path = "test_capture_fullscreen.png";
    frame.image.save(output_path).expect("Failed to save image");
    println!("  Saved to: {}\n", output_path);

    // 5. Capture first visible window
    if let Some(win) = windows.first() {
        println!("--- Capture Window: \"{}\" ---", win.title);
        match backend.capture_window(win.id) {
            Ok(frame) => {
                println!(
                    "  Captured: {}x{}",
                    frame.image.width(),
                    frame.image.height()
                );
                let output_path = "test_capture_window.png";
                frame.image.save(output_path).expect("Failed to save image");
                println!("  Saved to: {}", output_path);
            }
            Err(e) => println!("  Failed: {}", e),
        }
    }

    println!("\n=== All tests passed! ===");
}
