---
phase: 02-radio-abstraction-spi
plan: C
status: complete
---

# 02-C Summary

## Delivered

- Workspace crate `maverick-adapter-radio-spi` with Cargo feature `spi`; default workspace/CI builds omit libloragw.
- `SpiUplinkSource` implements `UplinkSource`; `next_batch` uses `spawn_blocking` with a **placeholder** poll (metadata check + sleep → `Idle`) until libloragw RX is vendored.
- `SpiConcentratorIngressBackend` + `UplinkBackendKind::ConcentratorSpi` for capability / logging identity.
- `radio_ingest_selection`: resolve `[radio]` from `lns-config.toml`, `build_uplink_source` (UDP vs SPI).
- `gwmp_loop` uses `Arc<dyn UplinkSource>`; `RuntimeCapabilityReport::build` reflects SPI vs UDP from file.
- `maverick-edge` feature `spi` enables the optional adapter dependency.

## Follow-up

- Vendor Semtech libloragw, `build.rs` + FFI, map `lgw_receive` packets → `UplinkObservation` (see `02-RESEARCH.md`).
