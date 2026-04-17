---
phase: "09"
title: hardware-auto-detection-spi-enable
status: complete
completed: "2026-04-17"
wave_count: 3
plans_total: 4
plans_complete: 4
---

## Phase 09 Summary: Hardware Auto-Detection & SPI Enable

### Objective
Extend RadioEnvironmentHints to detect SX1302/SX1303 SPI concentrator hardware on Linux hosts by probing `/dev/spidev*` device nodes. Add "auto" mode to RadioBackend that probes for SPI hardware and auto-enables SPI ingest when concentrator hardware is detected.

### Waves

| Wave | Plans | Description |
|------|-------|-------------|
| 1 | 09-A, 09-B | SPI hardware detection structs + Auto mode logic |
| 2 | 09-C | TUI/probe output integration |
| 3 | 09-D | Verification (manual testing on hardware) |

### What was built

**Wave 1 (09-A, 09-B):**
- `SpiHardwareHints` and `ConcentratorCandidate` structs for hardware detection
- `probe_spi_hardware()` function probing `/dev/spidev*` on Linux
- `RadioBackend::Auto` variant for automatic SPI/UDP selection
- `RadioIngestSelection::AutoSpi`/`AutoUdp` variants for resolved selections
- Validation and runtime probing for auto mode

**Wave 2 (09-C):**
- `spi_recommended_but_not_enabled()` method on `RuntimeCapabilityReport`
- SPI auto-enable guidance in `format_operator_summary()`
- Unit tests for SPI hardware detection

**Wave 3 (09-D):**
- Manual verification plan (execution deferred - requires physical SPI hardware)

### Key features

- **Best-effort detection**: SPI hardware detection never fails startup
- **Auto mode**: `radio.backend = "auto"` probes at runtime, no `spi_path` required
- **Graceful fallback**: When no SPI hardware found, UDP is used with informative log
- **Operator guidance**: Probe output shows actionable config snippets when SPI detected but not enabled

### Files modified

- `crates/maverick-runtime-edge/src/runtime_capabilities.rs`
- `crates/maverick-core/src/lns_config.rs`
- `crates/maverick-runtime-edge/src/radio_ingest_selection.rs`
- `crates/maverick-runtime-edge/src/ingest/gwmp_loop.rs`

### Verification

- `cargo build --package maverick-runtime-edge` passes
- `cargo build --package maverick-core` passes
- All tests pass (48 maverick-core, 3 maverick-runtime-edge)
- Plan 09-D requires manual testing on RAK Pi hardware with SPI

### Requirements addressed

- CORE-03: Hardware auto-detection
- CORE-04: Platform capability reporting
- RADIO-01, RADIO-02, RADIO-03, RADIO-04: SPI hardware detection and auto-enable

### Notes

- Plan 09-D (verification) has `autonomous: false` because it requires physical SPI hardware
- Manual testing steps documented in 09-D-PLAN.md
- On x86 dev machine (no SPI), probe correctly shows "SPI hardware: none detected"
