---
phase: 02-radio-abstraction-spi
status: partial
verification_date: 2026-04-16
verified_by: tbd
---

# Phase 2: Radio Abstraction & SPI — Verification

## Success Criteria Status

| # | Criterion | Status | Evidence |
|---|-----------|--------|----------|
| 1 | `UplinkSource` port trait is implemented by both the UDP adapter and the new SPI adapter — ingest loop code is unchanged when backend switches | ⚠️ PARTIAL | UDP adapter verified; SPI adapter uses placeholder (libloragw RX not yet integrated) |
| 2 | On a Raspberry Pi with RAK LoRa HAT, Maverick reads and persists uplinks directly from the SX1302/SX1303 without a Semtech packet forwarder running | ❌ NOT VERIFIED | Requires hardware testing |
| 3 | Radio backend (SPI or UDP) is selectable via config file — existing UDP path remains fully functional for dev and simulator use | ✅ VERIFIED | `[radio]` section in lns-config.toml + optional in docs |
| 4 | Hardware compatibility registry lists RAK Pi HAT as verified-supported; ships as a TOML file community can extend without code changes | ✅ VERIFIED | `docs/hardware-registry.toml` exists with RAK HAT entries |

## Verification Evidence

### Criterion 1 (Partial)
- `UplinkSource` trait defined in `crates/maverick-core/src/ports/uplink_source.rs`
- `GwmpUdpUplinkSource` implements trait (02-B-PLAN.md)
- `SpiRadioUplinkSource` exists as SPI adapter (02-C-PLAN.md)
- **Gap:** SPI adapter is a placeholder — requires libloragw RX integration

### Criterion 3
- `docs/lns-config.md`: optional `[radio]` section documented
- `docs/hardware-registry.toml`: hardware registry exists

### Criterion 4
- `docs/hardware-registry.toml` exists with schema_version and `boards` rows
- RAK HAT entries for aarch64 + armv7 present

## Gaps

- **SPI radio full implementation** — libloragw RX not integrated; SPI adapter is a stub
- **Hardware testing on RAK Pi** — Requires physical hardware and real radio environment

## Next Verification Required

After Phase 8 (Hardware Testing) completion, re-verify criteria 1 and 2 with physical hardware.

---
*Verification created: 2026-04-16*
