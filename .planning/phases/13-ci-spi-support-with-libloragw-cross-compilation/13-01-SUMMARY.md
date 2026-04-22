---
phase: 13
plan: "01"
title: "CI SPI Support with libloragw Cross-Compilation"
status: complete
completed: "2026-04-17"
autonomous: true
---

## Summary

Completed CI support for building SPI-enabled release binaries on ARM targets (aarch64, armv7) by vendoring the sx1302 HAL and teaching `build.rs` to detect cross-compilation sysroots.

### What Changed

1. **Vendored sx1302_hal sources** (`crates/maverick-adapter-radio-spi/libloragw/`)
   - Added git submodule `sx1302_hal` for complete HAL build.
   - Copied all missing libloragw source files and `tinymt32` for cross-compilation.
   - Added bash script to generate `config.h` in CI instead of relying on pre-built libs.
   - Prioritized local `inc/` directory and added sysroot header copies.

2. **`build.rs` cross-compilation detection**
   - Reads `CFLAGS_*` environment variables (e.g., `CFLAGS_aarch64_unknown_linux_gnu`, `CFLAGS_armv7_unknown_linux_gnueabihf`).
   - Extracts `--sysroot=` flags and passes them to `cc::Build`.
   - Fallback via `CARGO_BUILD_TARGET` + `SYSROOT_AARCH64` / `SYSROOT_ARMV7` env vars.
   - Native x86_64 builds remain unaffected (no sysroot needed).

3. **`release.yml` SPI feature gating**
   - ARM targets (`aarch64-unknown-linux-gnu`, `armv7-unknown-linux-gnueabihf`) now build with `--features spi`.
   - x86_64 build remains without SPI (avoids toolchain requirement).

4. **Verification**
   - `cargo check -p maverick-adapter-radio-spi` passes.
   - `cargo check -p maverick-runtime-edge --features spi` passes.
   - `cargo fmt --check` passes.
   - CI release pipeline produces SPI-enabled ARM binaries.

### Decisions

- Vendoring HAL sources avoids external download dependencies in CI and guarantees reproducible builds.
- Sysroot detection via `CFLAGS_*` is the primary mechanism because `cross` already sets these variables; fallback env vars support custom toolchains.

### Tech Debt / Notes

- Full SPI RX/TX on real ARM hardware is still pending (`RADIO-01` in Active requirements).
- Phase 09-D (Auto-Detection Verification) remains unexecuted; can be picked up when hardware is available.

---
*Completed: 2026-04-17*
