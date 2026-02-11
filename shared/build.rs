fn main() {
    #[cfg(target_os = "macos")]
    {
        println!("cargo:rerun-if-changed=src/codec/macos/vtbridge/vtbridge.m");
        println!("cargo:rerun-if-changed=src/codec/macos/vtbridge/vtbridge.h");

        cc::Build::new()
            .file("src/codec/macos/vtbridge/vtbridge.m")
            .flag("-fobjc-arc")
            .compile("vtbridge");

        println!("cargo:rustc-link-lib=framework=VideoToolbox");
        println!("cargo:rustc-link-lib=framework=CoreMedia");
        println!("cargo:rustc-link-lib=framework=CoreVideo");
        println!("cargo:rustc-link-lib=framework=Foundation");
        println!("cargo:rustc-link-lib=framework=QuartzCore");
    }
}
