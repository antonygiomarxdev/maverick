---
phase: "14"
name: fix-cross-compiled-spi-binary
status: planning
date: 2026-04-29
---

# Phase 14: Fix Cross-Compiled SPI Binary Initialization

**Gathered:** 2026-04-29
**Status:** Ready for planning
**Source:** GitHub Issue #10 + RAK Pi Debug Report

## Domain

### Phase Boundary

Fix the cross-compiled aarch64 release binary so it successfully initializes the SX1302/SX1250 SPI concentrator on Raspberry Pi + RAK2287 hardware.

### Root Cause (from debug session)

- **Native build on Pi:** Works (SPI init succeeds, `listen timeout` = expected)
- **Cross-compiled release (CI):** Fails at `lgw_rxrf_setconf chain 0 failed: -1`
- **Chip detection works:** `0x05` = SX1302 v0.5 is correctly read
- **Failure point:** SX1250 frontend configuration after chip detection

**Hypothesis:** Struct padding/layout or compiler optimization difference between `aarch64-linux-gnu-gcc` (CI) and native gcc on Pi causes incorrect SPI register values when writing SX1250 configuration.

## Decisions

### Locked Decisions
- Must fix cross-compilation without breaking native builds
- Must add CI verification to prevent regression
- Must maintain backward compatibility with existing lns-config.toml format
- Must not require hardware changes (concentrator, Pi, wiring)

### Implementation Approach
1. **Primary:** Add compile-time struct layout assertions (`static_assert`) in C code and Rust FFI bindings
2. **Secondary:** Compare `sizeof(struct lgw_conf_rxrf_s)` and field offsets between cross-compiled and native builds
3. **Verification:** Add QEMU-based SPI smoke test to CI (emulated SX1302 that responds to init sequence)
4. **Fallback:** If struct layout is not the issue, investigate SPI timing/speed differences

## Canonical References

- `docs/debug/rakpi-spi-failure-2026-04-29.md` — Full debug report with evidence
- `crates/maverick-adapter-radio-spi/build.rs` — Cross-compilation sysroot detection
- `crates/maverick-adapter-radio-spi/libloragw/` — Vendored HAL sources
- `.github/workflows/release.yml` — Release CI pipeline
- GitHub Issue #10 — Cross-compiled binary fails SPI initialization
- GitHub Issue #12 — Add SPI smoke test to CI

## Specific Ideas

### Struct Layout Verification
```c
// In libloragw C code or build.rs generated test
static_assert(sizeof(struct lgw_conf_rxrf_s) == EXPECTED_SIZE, "struct size mismatch");
static_assert(offsetof(struct lgw_conf_rxrf_s, freq_hz) == EXPECTED_OFFSET, "field offset mismatch");
```

### Rust FFI Binding Verification
```rust
// In Rust FFI bindings
assert_eq!(std::mem::size_of::<lgw_conf_rxrf_s>(), EXPECTED_SIZE);
assert_eq!(std::mem::align_of::<lgw_conf_rxrf_s>(), EXPECTED_ALIGNMENT);
```

### QEMU SPI Smoke Test
- Use QEMU with virtio-spi or custom device model
- Emulate SX1302 response to `lgw_start()` sequence
- Run in CI after cross-compiled build
- Verify no `lgw_start failed` or `lgw_rxrf_setconf failed` errors

## Deferred Ideas

- Full hardware-in-the-loop CI on real RAK Pi (too slow for PR builds)
- SPI speed/timing adjustments (only if struct layout is not the root cause)
- bindgen auto-generation of Rust structs from C headers (future improvement)

---

*Phase: 14-fix-cross-compiled-spi-binary*
*Context gathered: 2026-04-29 via debug session + GitHub issue*
