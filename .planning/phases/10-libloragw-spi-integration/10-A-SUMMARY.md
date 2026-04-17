# Phase 10 Plan A: libloragw Vendor Foundation — Summary

## What Was Built

Phase 10 Plan A establishes the complete libloragw C foundation for SX1302/SX1303 direct hardware integration.

## Implementation

### A-1: Vendor sx1302_hal as git submodule
- Added `https://github.com/lora-net/sx1302_hal.git` as `libloragw/` submodule
- Checked out tag `V2.1.0`
- Generated `libloragw/libloragw/inc/config.h` via `make inc/config.h` to support bindgen

### A-2: Update Cargo.toml
```toml
[build-dependencies]
cc = { version = "1", optional = true }

[features]
default = []
spi = ["dep:cc"]
```

### A-3: Create build.rs
- Uses `std::env::var("CARGO_FEATURE_SPI")` (NOT `cfg!()`) to detect feature
- Early returns when feature not set (x86 CI lint)
- Compiles 10 C sources from `libloragw/libloragw/src/`
- Includes `-I libloragw/libloragw/inc` for header resolution
- Links against `libm` and `librt`

### A-4: Generate bindings.rs
- Generated via `bindgen` with allowlist for `lgw_.*` types/functions/vars
- 37 `lgw_*` functions including:
  - `lgw_start()` / `lgw_stop()` — HAL lifecycle
  - `lgw_receive(max_pkt, pkt_data)` — RX path
  - `lgw_board_setconf()`, `lgw_rxrf_setconf()`, `lgw_sx1302_rx_setconf()` — configuration
- Key structs: `lgw_pkt_rx_s`, `lgw_board_conf_s`, `lgw_rxrf_conf_s`, `lgw_sx1302_rx_conf_s`

## Key Decisions

- **Submodule path**: `libloragw/libloragw/` (sx1302_hal has nested `libloragw/` directory structure)
- **config.h generation**: Must run `make inc/config.h` in `libloragw/libloragw/` before bindgen
- **bindgen include paths**: GCC stdbool.h at `/usr/lib/gcc/x86_64-linux-gnu/13/include`

## Verification

| Check | Result |
|-------|--------|
| `libloragw/` is git submodule | ✓ |
| `libloragw/libloragw/inc/loragw_hal.h` exists | ✓ |
| `build.rs` uses `CARGO_FEATURE_SPI` env var | ✓ |
| `Cargo.toml` has `cc` optional build-dep | ✓ |
| `bindings.rs` contains `lgw_receive` | ✓ |
| `bindings.rs` contains `lgw_start`, `lgw_stop` | ✓ |

## Commits

- `7ad1256` chore: add cc build-dep to Cargo.toml for libloragw C compilation
- `18994d7` feat(radio-spi): vendor sx1302_hal and generate libloragw FFI bindings

## Notes

- bindings.rs generated on x86 — ARM cross-compile target not installed
- Struct layout should be correct via `--offsetof-in-repr` equivalent behavior
- Submodule checked at V2.1.0 tag — consider pinning to commit SHA for reproducibility
