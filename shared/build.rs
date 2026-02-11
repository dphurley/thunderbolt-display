fn main() {
    #[cfg(target_os = "macos")]
    {
        println!("cargo:rerun-if-changed=src/codec/macos/vtbridge/vtbridge.m");
        println!("cargo:rerun-if-changed=src/codec/macos/vtbridge/vtbridge.h");
        println!("cargo:rerun-if-changed=src/platform/macos/virtual_display/virtual_display.m");
        println!("cargo:rerun-if-changed=src/platform/macos/virtual_display/virtual_display.h");

        cc::Build::new()
            .file("src/codec/macos/vtbridge/vtbridge.m")
            .flag("-fobjc-arc")
            .compile("vtbridge");

        cc::Build::new()
            .file("src/platform/macos/virtual_display/virtual_display.m")
            .flag("-fobjc-arc")
            .compile("virtual_display");

        println!("cargo:rustc-link-lib=framework=VideoToolbox");
        println!("cargo:rustc-link-lib=framework=CoreMedia");
        println!("cargo:rustc-link-lib=framework=CoreVideo");
        println!("cargo:rustc-link-lib=framework=Foundation");
        println!("cargo:rustc-link-lib=framework=QuartzCore");
        println!("cargo:rustc-link-lib=framework=CoreGraphics");
    }
}
