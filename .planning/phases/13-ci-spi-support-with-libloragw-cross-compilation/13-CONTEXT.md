# Phase 13: CI SPI Support with libloragw Cross-Compilation — Context

**Gathered:** 2026-04-17
**Status:** Ready for planning

<domain>
## Phase Boundary

Make release CI build binaries WITH SPI support so auto-update works for SPI hardware devices. Currently CI builds are SPI-disabled because libloragw C sources are not available to the ARM cross-compiler. The goal is to have working SPI binaries for ARM targets (aarch64, armv7) in release artifacts.

**Out of scope:** Runtime SPI hardware testing, SPI TX/downlink, modifying radio adapter logic

</domain>

<decisions>
## Implementation Decisions

### Source Delivery (D-01)
- **libloragw sources are already vendored** in `crates/maverick-adapter-radio-spi/libloragw/` — no submodule or external fetch needed
- CI must clone the full repo so the `libloragw/` subtree is available during ARM cross-compilation

### Cross-Compilation Fix (D-02)
- Current `build.rs` has hardcoded relative paths that work on x86 but fail for ARM cross-compilation
- Need to detect cross-compilation via `CARGO_TARGET_*` environment variables and pass `--sysroot` to ARM gcc
- Already set in release.yml for aarch64 (`--sysroot=/usr/aarch64-linux-gnu`) and armv7 (`--sysroot=/usr/arm-linux-gnueabihf`) via `CFLAGS_*` env vars
- build.rs needs to read `CFLAGS_*` from environment and pass to `cc::Build`

### Feature Gate (D-03)
- SPI feature (`maverick-adapter-radio-spi/spi`) must only build on ARM targets in CI
- x86_64 builds should NOT activate SPI (C sources require ARM toolchain for cross-compile)
- release.yml already sets up cross-toolchains for aarch64 and armv7 but NOT x86_64 (native build doesn't need it)

### CI Build Order (D-04)
- Step 1: Checkout sources (includes libloragw subtree)
- Step 2: Setup Rust cross-targets
- Step 3: Install cross-compilation toolchains (aarch64/armv7)
- Step 4: Build with `CARGO_FEATURE_SPI=1` passed to cargo for ARM targets only

### Sysroot Detection in build.rs (D-05)
- Use `std::env::var("CFLAGS_aarch64_unknown_linux_gnu")` etc. to detect cross-compilation context
- Parse `--sysroot=...` from CFLAGS and pass to cc::Build
- Fall back to local include paths for native builds

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Prior phase context
- `.planning/phases/10-libloragw-spi-integration/10-CONTEXT.md` — Phase 10 integration decisions, build.rs structure
- `.planning/phases/12-release-ci-hardening-and-update-url-configuration/12-CONTEXT.md` — Phase 12 release workflow
- `.planning/PROJECT.md` — offline-first, multi-arch builds for ARM gateways

### CI/CD references
- `.github/workflows/release.yml` — existing release workflow to extend
- `crates/maverick-adapter-radio-spi/build.rs` — current build.rs that needs cross-compilation fix

### Source tree
- `crates/maverick-adapter-radio-spi/libloragw/libloragw/inc/` — libloragw headers
- `crates/maverick-adapter-radio-spi/libloragw/libloragw/src/` — libloragw C sources (already vendored)

</canonical_refs>

<codebase_context>
## Existing Code Insights

### Current build.rs structure
- Hardcodes paths relative to crate root: `libloragw/libloragw/src/loragw_hal.c`
- Uses `cc::Build` with fixed include: `.include("libloragw/libloragw/inc")`
- No cross-compilation awareness — fails when ARM gcc is invoked with `--sysroot`

### release.yml current state
- Sets `CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc` etc.
- Sets `CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc`
- Sets `CFLAGS_aarch64_unknown_linux_gnu=--sysroot=/usr/aarch64-linux-gnu`
- Same pattern for armv7
- cargo build commands do NOT pass any SPI-related flags currently

### libloragw vendored structure
- `crates/maverick-adapter-radio-spi/libloragw/libloragw/src/*.c` — all HAL sources
- `crates/maverick-adapter-radio-spi/libloragw/libloragw/inc/*.h` — public headers

</codebase_context>

<specifics>
## Specific Ideas

- build.rs: detect `CFLAGS_*` env vars, extract `--sysroot`, pass to cc::Build
- release.yml: pass `CARGO_FEATURE_SPI=1` to cargo build for ARM targets
- Possibly add `--features maverick-adapter-radio-spi/spi` or use environment variable in build.rs
- The SPI feature already gates the C compilation via `std::env::var("CARGO_FEATURE_SPI")` in build.rs

</specifics>

<deferred>
## Deferred Ideas

None — phase scope is well-defined and bounded.

</deferred>

---

*Phase: 13-ci-spi-support-with-libloragw-cross-compilation*
*Context gathered: 2026-04-17*
*[auto] All gray areas selected with recommended defaults — no user discussion needed for pure CI/compiler infrastructure*