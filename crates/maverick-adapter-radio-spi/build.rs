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
        .flag("-Wno-sign-compare")
        .compile("loragw");

    println!("cargo:rustc-link-lib=m");
    println!("cargo:rustc-link-lib=rt");
}
