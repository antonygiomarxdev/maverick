---
phase: "09"
plan: C
title: TUI Auto-Detection Integration
status: complete
completed: "2026-04-17"
wave: 2
depends_on:
  - 09-A
  - 09-B
---

## Summary

Surfaced auto-detected SPI hardware in probe command output. When SPI hardware is detected but no `[radio]` config exists, operators now receive clear guidance on enabling SPI.

## What was built

- **spi_recommended_but_not_enabled() method**: Returns true when SPI hardware detected but UDP is active
- **SPI auto-enable guidance in format_operator_summary()**: Actionable configuration snippets shown when SPI detected but not enabled
- **Unit tests for SPI hardware detection**: Tests that verify detection is best-effort and never fails startup

## Key decisions

- Guidance shown ONLY when SPI hardware detected AND current ingest is UDP
- Guidance includes both "auto" mode and explicit SPI path options
- No confusing messages when SPI is already enabled
- Unit tests verify detection works on platforms without `/dev/spidev*`

## Files modified

- `crates/maverick-runtime-edge/src/runtime_capabilities.rs`

## Verification

- `cargo build --package maverick-runtime-edge` passes
- `cargo test --package maverick-runtime-edge -- runtime_capabilities` passes (2 tests)
