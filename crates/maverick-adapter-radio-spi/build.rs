fn main() {
    if std::env::var("CARGO_FEATURE_SPI").is_err() {
        return;
    }

    let sources = [
        "libloragw/libloragw/src/loragw_hal.c",
        "libloragw/libloragw/src/loragw_spi.c",
        "libloragw/libloragw/src/loragw_reg.c",
        "libloragw/libloragw/src/loragw_sx1302.c",
        "libloragw/libloragw/src/loragw_sx1302_rx.c",
        "libloragw/libloragw/src/loragw_sx1302_timestamp.c",
        "libloragw/libloragw/src/loragw_sx125x.c",
        "libloragw/libloragw/src/loragw_sx1250.c",
        "libloragw/libloragw/src/loragw_aux.c",
        "libloragw/libloragw/src/loragw_com.c",
    ];

    let mut build = cc::Build::new();
    for s in &sources {
        build.file(s);
    }
    build
        .include("libloragw/libloragw/inc")
        .flag("-Wno-unused-parameter")
        .flag("-Wno-sign-compare");

    // Detect cross-compilation target
    let target = std::env::var("CARGO_BUILD_TARGET").ok();

    // Always include the local libloragw inc dir first
    build.include("libloragw/libloragw/inc");

    // Check CFLAGS_* env vars for --sysroot (set by release.yml cross-compilation setup)
    // These are the primary mechanism since release.yml sets CFLAGS_aarch64_* and CFLAGS_armv7_*
    if let Some(cflags) = std::env::var("CFLAGS").ok() {
        for part in cflags.split_whitespace() {
            if part.starts_with("--sysroot=") {
                build.flag(part);
            }
        }
    }

    // Check target-specific CFLAGS (release.yml sets these for ARM cross-compilation)
    if let Some(cflags) = std::env::var("CFLAGS_aarch64_unknown_linux_gnu").ok() {
        for part in cflags.split_whitespace() {
            if part.starts_with("--sysroot=") {
                build.flag(part);
            }
        }
    }
    if let Some(cflags) = std::env::var("CFLAGS_armv7_unknown_linux_gnueabihf").ok() {
        for part in cflags.split_whitespace() {
            if part.starts_with("--sysroot=") {
                build.flag(part);
            }
        }
    }

    // Also detect via CARGO_BUILD_TARGET for direct sysroot flag usage
    if let Some(ref t) = target {
        if t.contains("aarch64") {
            if let Ok(sysroot) = std::env::var("AARCH64_UNKNOWN_LINUX_GNU_SYSROOT")
                .or_else(|_| std::env::var("SYSROOT_AARCH64"))
            {
                build.flag(&format!("--sysroot={}", sysroot));
            }
        } else if t.contains("armv7") {
            if let Ok(sysroot) = std::env::var("ARV7_UNKNOWN_LINUX_GNUEABIHF_SYSROOT")
                .or_else(|_| std::env::var("SYSROOT_ARMV7"))
            {
                build.flag(&format!("--sysroot={}", sysroot));
            }
        }
    }

    build
        .flag("-Wno-unused-parameter")
        .flag("-Wno-sign-compare");

    build.compile("loragw");

    println!("cargo:rustc-link-lib=m");
    println!("cargo:rustc-link-lib=rt");
}
