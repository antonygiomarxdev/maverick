---
phase: 02-radio-abstraction-spi
plan: A
status: complete
---

# 02-A Summary

## Delivered

- `UplinkSource` async trait + `UplinkReceive` enum (`Idle` vs `Observations`) in `maverick-core::ports` — distinguishes UDP read timeout from a datagram with zero `rxpk` rows.
- `RadioBackend`, `RadioConfig`, optional `LnsConfigDocument.radio` with `#[serde(default)]`; `validate()` requires `spi_path` when `backend = spi`.
- Unit tests for SPI path validation.

## Files

- `crates/maverick-core/src/ports/uplink_source.rs` (new)
- `crates/maverick-core/src/ports/mod.rs`
- `crates/maverick-core/src/lns_config.rs`

## Note

Plan text originally suggested `AppResult<Vec<UplinkObservation>>`; implementation uses `UplinkReceive` to preserve ingest JSON counters.
