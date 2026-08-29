use platform_windows::overlay::{OverlayResult, WindowsOverlay};

fn main() {
    println!("FalconShot Overlay PoC");
    println!("Drag to select a region (release captures it). Esc to cancel.");
    println!("Starting overlay in 2 seconds...");
    std::thread::sleep(std::time::Duration::from_secs(2));

    let mut overlay = WindowsOverlay::new();
    match overlay.show_and_select() {
        Ok(OverlayResult::Selected(rect, image)) => {
            println!(
                "Selected region: {}x{} at ({},{})",
                rect.width, rect.height, rect.x, rect.y
            );
            println!("Captured image: {}x{} px", image.width(), image.height());
            let out = std::env::temp_dir().join("falconshot_overlay_test.png");
            image.save(&out).ok();
            println!("Saved to {}", out.display());
        }
        Ok(OverlayResult::Cancelled) => {
            println!("Selection cancelled.");
        }
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }
}
