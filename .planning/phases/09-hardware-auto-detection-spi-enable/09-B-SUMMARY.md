---
phase: "09"
plan: B
title: Auto-Enable SPI Logic
status: complete
completed: "2026-04-17"
wave: 1
depends_on:
  - 09-A
---

## Summary

Added "auto" mode to RadioBackend that probes for SPI hardware and auto-enables SPI ingest when concentrator hardware is detected, without requiring manual `[radio]` config. Falls back to UDP when no SPI hardware found.

## What was built

- **RadioBackend::Auto variant**: New enum variant for automatic backend selection
- **RadioConfig::validate() update**: Allows Auto mode without requiring spi_path
- **AutoSpi/AutoUdp variants in RadioIngestSelection**: Handle resolved auto mode selections
- **resolve_radio_ingest() update**: Probes SPI hardware when Auto mode is configured
- **build_uplink_source() update**: Handles AutoSpi and AutoUdp variants with appropriate behavior
- **gwmp_loop.rs updates**: Handle new RadioIngestSelection variants in listen_label and trace_ingest_identity
- **RuntimeCapabilityReport::build() update**: Maps AutoSpi/AutoUdp to appropriate UplinkBackendKind

## Key decisions

- Auto mode requires no spi_path in config (probed at runtime)
- When SPI hardware detected, uses first concentrator candidate
- When no SPI hardware found, falls back to UDP with informative log message
- SPI feature flag gating: AutoSpi only works when `spi` feature is enabled

## Files modified

- `crates/maverick-core/src/lns_config.rs`
- `crates/maverick-runtime-edge/src/radio_ingest_selection.rs`
- `crates/maverick-runtime-edge/src/runtime_capabilities.rs`
- `crates/maverick-runtime-edge/src/ingest/gwmp_loop.rs`

## Verification

- `cargo build --package maverick-runtime-edge` passes
- `cargo build --package maverick-core` passes
- All maverick-runtime-edge tests pass
