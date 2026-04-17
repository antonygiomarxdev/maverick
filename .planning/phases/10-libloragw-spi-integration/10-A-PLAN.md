---
wave: 1
depends_on: []
requirements_addressed: [RADIO-01, RADIO-02]
files_modified:
  - crates/maverick-adapter-radio-spi/Cargo.toml
  - crates/maverick-adapter-radio-spi/build.rs
  - crates/maverick-adapter-radio-spi/libloragw/submodule (sx1302_hal)
---
# Phase 10: Vendor libloragw C Sources — Plan A

**Wave 1:** Foundation — vendor sx1302_hal in-tree, add build.rs C compilation, generate bindings

## Context

- Phase 2 established the in-tree C vendoring pattern with `cc` in `build.rs`
- Phase 2 research confirmed `loragw-hal` does NOT exist on crates.io — must vendor
- Phase 2 confirmed: `std::env::var("CARGO_FEATURE_SPI")` (NOT `cfg!(feature = "spi")`) in build.rs
- Phase 9 added `RadioBackend::Auto` and `RadioIngestSelection::AutoSpi` — Phase 10 wires SPI to real hardware

## Objective

Set up the complete libloragw C foundation: vendored sx1302_hal submodule, `build.rs` that compiles it when `spi` feature is active, and `bindings.rs` with pre-generated FFI for Rust interop.

## Tasks

### A-1: Vendor sx1302_hal as git submodule

**Action:** In `crates/maverick-adapter-radio-spi/`, add sx1302_hal as a git submodule:

```bash
git submodule add https://github.com/lora-net/sx1302_hal.git libloragw
cd libloragw && git checkout v2.1.0  # Or pinned commit SHA
```

Resulting structure:
```
maverick-adapter-radio-spi/
  libloragw/
    inc/          # loragw_hal.h, loragw_reg.h, lgw_sx1302.h, etc.
    src/          # loragw_hal.c, loragw_spi.c, loragw_reg.c, loragw_sx1302.c,
                  # loragw_sx1302_rx.c, loragw_sx1302_timestamp.c, loragw_sx125x.c,
                  # loragw_sx1250.c, loragw_aux.c, loragw_com.c
    Makefile
    README.md
```

**read_first:**
- `.planning/phases/02-radio-abstraction-spi/02-RESEARCH.md` — D-06, D-08, pattern 2 (build.rs C compilation), pitfall 3 (lgw_start/lgw_stop symmetry)
- `crates/maverick-adapter-radio-spi/README.md` — existing crate documentation
- `crates/maverick-adapter-radio-spi/Cargo.toml` — current state before modification

**acceptance_criteria:**
- `libloragw/` directory is a git submodule pointing to `https://github.com/lora-net/sx1302_hal.git`
- `libloragw/inc/loragw_hal.h` exists (required by bindings generation)
- `libloragw/src/` contains `loragw_hal.c`, `loragw_spi.c`, `loragw_reg.c`, `loragw_sx1302.c`, `loragw_sx1302_rx.c`, `loragw_sx1302_timestamp.c`, `loragw_sx125x.c`, `loragw_sx1250.c`, `loragw_aux.c`, `loragw_com.c`
- `libloragw/.git` file exists (submodule marker)

---

### A-2: Update Cargo.toml — add cc build-dependency, libm/librt link

**Action:** Update `crates/maverick-adapter-radio-spi/Cargo.toml`:

```toml
[build-dependencies]
cc = { version = "1", optional = true }

[features]
default = []
spi = ["dep:cc"]

[dependencies]
maverick-core = { workspace = true }
async-trait = { workspace = true }
tokio = { workspace = true, features = ["rt", "sync"] }
```

**read_first:**
- `crates/maverick-adapter-radio-spi/Cargo.toml` — current state
- `crates/maverick-adapter-radio-spi/build.rs` — does not exist yet (will be created)

**acceptance_criteria:**
- `[build-dependencies]` section added with `cc = { version = "1", optional = true }`
- `spi = ["dep:cc"]` in `[features]` — cc compiled only when spi feature active
- No `bindgen` in `[build-dependencies]` — bindings are pre-generated, not regenerated at build time

---

### A-3: Create build.rs — compile libloragw C when CARGO_FEATURE_SPI is set

**Action:** Create `crates/maverick-adapter-radio-spi/build.rs`:

```rust
fn main() {
    // CARGO_FEATURE_SPI is set by Cargo when the "spi" feature is active.
    // NOT cfg!(feature = "spi") — that evaluates host features, not crate being built.
    if std::env::var("CARGO_FEATURE_SPI").is_err() {
        return; // x86 / CI lint job: skip C compilation entirely
    }

    let sources = [
        "libloragw/src/loragw_hal.c",
        "libloragw/src/loragw_spi.c",
        "libloragw/src/loragw_reg.c",
        "libloragw/src/loragw_sx1302.c",
        "libloragw/src/loragw_sx1302_rx.c",
        "libloragw/src/loragw_sx1302_timestamp.c",
        "libloragw/src/loragw_sx125x.c",
        "libloragw/src/loragw_sx1250.c",
        "libloragw/src/loragw_aux.c",
        "libloragw/src/loragw_com.c",
    ];

    let mut build = cc::Build::new();
    for s in &sources {
        build.file(s);
    }
    build
        .include("libloragw/inc")
        .flag("-Wno-unused-parameter")
        .flag("-Wno-sign-compare")
        .compile("loragw");

    // libloragw links against libm and librt
    println!("cargo:rustc-link-lib=m");
    println!("cargo:rustc-link-lib=rt");
}
```

**read_first:**
- `.planning/phases/02-radio-abstraction-spi/02-RESEARCH.md` — Pattern 2 (build.rs C compilation), CRITICAL note about `cfg!(feature = "spi")` not working in build.rs

**acceptance_criteria:**
- `build.rs` exists and uses `std::env::var("CARGO_FEATURE_SPI")` NOT `cfg!()`
- All 10 C source files listed in build array
- `build.include("libloragw/inc")` so loragw_hal.h is found
- `compile("loragw")` outputs `libloragw.a` static archive
- `println!("cargo:rustc-link-lib=m")` and `println!("cargo:rustc-link-lib=rt")` for link deps
- `cargo build --features spi` on x86 returns early without compiling C (feature inactive)

---

### A-4: Generate bindings.rs via bindgen (on ARM target or with --target)

**Action:** Generate FFI bindings for `libloragw/inc/loragw_hal.h` using bindgen. This MUST be done on ARM or with `--target aarch64-unknown-linux-gnu` to avoid struct layout mismatches.

```bash
# Option 1: On ARM hardware (RAK Pi)
cd crates/maverick-adapter-radio-spi
bindgen libloragw/inc/loragw_hal.h \
  --output src/bindings.rs \
  --offsetof-in-repr \
  --allowlist-type lgw_.* \
  --allowlist-fn lgw_.* \
  --allowlist-var LGW_.*

# Option 2: Cross-compile target (requires aarch64 target installed)
rustup target add aarch64-unknown-linux-gnu
bindgen libloragw/inc/loragw_hal.h \
  --target aarch64-unknown-linux-gnu \
  --output src/bindings.rs \
  --offsetof-in-repr \
  --allowlist-type lgw_.* \
  --allowlist-fn lgw_.* \
  --allowlist-var LGW_.*
```

The generated `bindings.rs` should include:
- `lgw_start() -> i32`
- `lgw_stop() -> i32`
- `lgw_receive(max_pkt: u8, pkt_data: *mut lgw_pkt_rx_s) -> i32`
- `lgw_board_setconf(conf: *const lgw_board_conf_s) -> i32`
- `lgw_rxrf_setconf(chain: u8, conf: *const lgw_rxrf_conf_s) -> i32`
- `lgw_sx1302_rx_setconf(chain: u8, conf: *const lgw_sx1302_rx_conf_s) -> i32`
- `LGW_LORA_RX`, `LGW_HAL_ERROR`, `LGW_SUCCESS` constants
- `lgw_pkt_rx_s` struct with fields: `freq_hz`, `rf_chain`, `modulation`, `datarate`, `rssic`, `snr`, `size`, `payload[256]`, `count`, `utc`, `rxtime`, `crc`, `opt_width`
- `lgw_board_conf_s`, `lgw_rxrf_conf_s`, `lgw_sx1302_rx_conf_s` structs

**read_first:**
- `.planning/phases/02-radio-abstraction-spi/02-RESEARCH.md` — Pitfall 2 (bindgen padding on ARM cross-compile), Open Questions item 3
- `libloragw/inc/loragw_hal.h` — header file to generate bindings from

**acceptance_criteria:**
- `src/bindings.rs` exists with `bindgen` marker comment at top
- `bindings.rs` contains `extern "C" { fn lgw_start(...)` etc.
- `bindings.rs` contains struct definitions for `lgw_pkt_rx_s`, `lgw_board_conf_s`, `lgw_rxrf_conf_s`, `lgw_sx1302_rx_conf_s`
- `bindings.rs` does NOT use `__bindgen_padding_*` fields for `lgw_pkt_rx_s` (padding should be in repr(C) blocks via `--offsetof-in-repr`)
- `bindings.rs` was generated with `--target aarch64-*` OR on real ARM hardware

---

## Verification

1. **Submodule exists:** `ls libloragw/inc/loragw_hal.h`
2. **Cargo.toml correct:** `grep -A5 '\[features\]' Cargo.toml | grep spi`
3. **build.rs correct:** `grep 'CARGO_FEATURE_SPI' build.rs`
4. **build.rs early-return:** `CARGO_FEATURE_SPI= cargo build --features spi` on x86 returns early
5. **bindings.rs exists:** `ls src/bindings.rs`
6. **bindings contain lgw_receive:** `grep 'fn lgw_receive' src/bindings.rs`

## Notes

- bindings.rs generation requires ARM target or hardware — generate once, check in
- `libloragw/` is a git submodule — clone/checkout must happen at `git clone --recursive` or `git submodule update --init --recursive`
- If bindings.rs is generated on x86, it may have struct layout mismatches for ARM — warn in README