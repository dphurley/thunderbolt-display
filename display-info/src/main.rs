#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("display-info is only supported on macOS");
}

#[cfg(target_os = "macos")]
fn main() {
    let displays = shared::platform::macos::display_info::list_displays();
    if displays.is_empty() {
        eprintln!("no displays detected");
        return;
    }

    for display in displays {
        let main_flag = if display.is_main == 1 { "main" } else { "" };
        println!(
            "id={} {}x{} {}",
            display.display_id, display.width, display.height, main_flag
        );
    }
}
