---
phase: "03"
plan: "02"
type: execute
wave: 1
autonomous: true
subsystem: protocol-security
tags: [udp, mic, spi, integration-tests, contract]
dependency_graph:
  requires: []
  provides:
    - UDP MIC extraction integration tests
    - SPI adapter parsing contract documentation
  affects:
    - gwmp
    - spi_uplink
tech_stack: [rust, udp, spi, lorawan]
key_files:
  created: []
  modified:
    - crates/maverick-adapter-radio-udp/src/gwmp.rs
    - crates/maverick-adapter-radio-spi/src/spi_uplink.rs
decisions:
  - "SPI adapter contract documented with PARSING_CONTRACT documentation block"
  - "Contract test marked as #[ignore] pending libloragw integration"
metrics:
  duration: "~5 minutes"
  completed_date: "2026-04-17"
  tasks_committed: 1
  files_modified: 2
---

# Phase 03 Plan 02: Integration Tests — Summary

**One-liner:** Add UDP MIC extraction integration tests and document SPI adapter parsing contract for future libloragw integration.

## What Was Built

### UDP Adapter MIC Extraction Tests (gwmp.rs)

Added 3 integration tests verifying the full pipeline from base64-encoded LoRaWAN frame to `UplinkObservation` with correctly extracted MIC fields:

1. **`full_pipeline_valid_frame_with_mic`** — Constructs a known LoRaWAN uplink frame manually and verifies all extracted fields (DevAddr, FCnt, FPort, payload, wire_mic, phy_without_mic)

2. **`mic_extraction_from_known_frame`** — Hardcodes a raw PHY frame with known MIC bytes and verifies exact extraction

3. **`phy_without_mic_correct_length`** — Verifies `phy_without_mic.len() == raw.len() - 4` for off-by-one error detection

### SPI Adapter Parsing Contract (spi_uplink.rs)

Added `PARSING_CONTRACT` documentation comment block at top of file explaining:
- Required UplinkObservation fields for libloragw integration
- Critical requirement: `wire_mic` and `phy_without_mic` MUST be extracted
- Warning: without these fields, MIC verification receives zeros and ALL frames are rejected

Added `#[cfg(test)]` module with `spi_adapter_parsing_contract` test (marked `#[ignore]` pending libloragw integration).

## Test Counts

| Category | Tests Added | Status |
|----------|-------------|--------|
| UDP MIC extraction | 3 | All pass |
| SPI contract | 1 (ignored) | Pending libloragw |

## Deviations from Plan

None — plan executed exactly as written.

## Verification

```bash
cargo test -p maverick-adapter-radio-udp --lib
cargo test -p maverick-adapter-radio-spi --lib --features spi
cargo clippy -p maverick-adapter-radio-udp -p maverick-adapter-radio-spi --all-features -- -D warnings
```
