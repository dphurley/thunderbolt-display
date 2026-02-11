#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("virtual-display is only supported on macOS");
}

#[cfg(target_os = "macos")]
fn main() {
    use shared::platform::macos::virtual_display::VirtualDisplay;

    let config = match parse_args() {
        Ok(config) => config,
        Err(message) => {
            eprintln!("{message}");
            print_usage();
            std::process::exit(1);
        }
    };

    let display = match VirtualDisplay::create(
        config.width,
        config.height,
        config.ppi,
        config.hi_dpi,
        &config.name,
    ) {
        Ok(display) => display,
        Err(error) => {
            eprintln!("failed to create virtual display: {error}");
            std::process::exit(1);
        }
    };

    eprintln!("created virtual display with id {}", display.display_id());
    eprintln!("press Ctrl-C to stop");
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

#[cfg(target_os = "macos")]
#[derive(Debug)]
struct Config {
    width: u32,
    height: u32,
    ppi: u32,
    hi_dpi: bool,
    name: String,
}

#[cfg(target_os = "macos")]
fn parse_args() -> Result<Config, String> {
    let mut width: u32 = 1920;
    let mut height: u32 = 1080;
    let mut ppi: u32 = 110;
    let mut hi_dpi = false;
    let mut name: String = "Thunderbolt Display".to_string();

    let mut args = std::env::args().skip(1);
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--width" => {
                let value = args.next().ok_or("missing --width value")?;
                width = value.parse().map_err(|_| "invalid width")?;
            }
            "--height" => {
                let value = args.next().ok_or("missing --height value")?;
                height = value.parse().map_err(|_| "invalid height")?;
            }
            "--ppi" => {
                let value = args.next().ok_or("missing --ppi value")?;
                ppi = value.parse().map_err(|_| "invalid ppi")?;
            }
            "--hi-dpi" => {
                hi_dpi = true;
            }
            "--name" => {
                let value = args.next().ok_or("missing --name value")?;
                name = value;
            }
            "--help" | "-h" => {
                return Err("".to_string());
            }
            _ => return Err(format!("unknown argument: {argument}")),
        }
    }

    Ok(Config {
        width,
        height,
        ppi,
        hi_dpi,
        name,
    })
}

#[cfg(target_os = "macos")]
fn print_usage() {
    eprintln!(
        "usage: virtual-display [--width N --height N --ppi N --hi-dpi --name STRING]"
    );
}
