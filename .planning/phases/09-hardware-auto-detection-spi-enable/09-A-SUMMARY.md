---
phase: "09"
plan: A
title: SPI Hardware Detection
status: complete
completed: "2026-04-17"
wave: 1
---

## Summary

Extended RadioEnvironmentHints to detect SX1302/SX1303 SPI concentrator hardware on Linux hosts by probing `/dev/spidev*` device nodes.

## What was built

- **SpiHardwareHints struct**: Contains available_devices, concentrator_candidates, and notes fields
- **ConcentratorCandidate struct**: Tracks SPI path, matched board, and concentrator model
- **spi_hardware field added to RadioEnvironmentHints**: Optional field for SPI hardware detection results
- **probe_spi_hardware() function**: Public function that probes `/dev/spidev*` on Linux, returns `Option<SpiHardwareHints>`
- **SPI hardware display in format_operator_summary()**: Shows available devices and concentrator candidates

## Key decisions

- SPI hardware detection is best-effort (never fails startup)
- Detection runs on every startup / config reload via `RadioEnvironmentHints::probe()`
- Public `probe_spi_hardware()` function allows reuse by radio_ingest_selection

## Files modified

- `crates/maverick-runtime-edge/src/runtime_capabilities.rs`

## Verification

- `cargo build --package maverick-runtime-edge` passes
- Tests for SPI hardware detection pass
