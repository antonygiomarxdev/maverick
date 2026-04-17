---
plan: 01-B
phase: 01-protocol-correctness
status: complete
tasks_completed: 2
tasks_total: 2
requirements_covered:
  - PROT-02
  - PROT-05
---

## Summary

Implemented 32-bit FCnt reconstruction, fixed region inference shadowing bug, and updated GWMP parser for new `UplinkObservation` fields.

## Tasks

### Task B-1: FCnt 32-bit extension in protocol module
- Added `FcntError` enum (`Duplicate`, `GapExceeded`) to `lorawan_10x_class_a.rs`
- Added `RejectFcntGapExceeded` variant to `ProtocolDecision` in `capability.rs`
- Implemented `LoRaWAN10xClassA::extend_fcnt(wire_u16: u16, session_fcnt: u32) -> Result<u32, FcntError>` with `MAX_FCNT_GAP = 16384` per LoRaWAN §4.3.1.5
- Updated `validate_uplink` to call `extend_fcnt` and gate on result
- Updated `sample_observation` test helper: `f_cnt: u16`, added `wire_mic`/`phy_without_mic` zero fields
- Added 4 unit tests for `extend_fcnt`: no-rollover, rollover at 16-bit boundary, duplicate rejected, gap exceeded

### Task B-2: Fix region inference and update GWMP parser
- Fixed `infer_region`: AS923 arm first (most specific), AU915 second, US915 third (`902.0..915.0` exclusive upper to avoid overlap) — resolves AU915/AS923 shadowing by US915
- Updated `parse_lorawan_payload` return type: `(DevAddr, u16, u8, Vec<u8>, [u8; 4], Vec<u8>)` — now returns `wire_mic` and `phy_without_mic`
- Updated `rxpk_to_observation` to populate all `UplinkObservation` fields including `wire_mic`, `phy_without_mic`
- Added 3 region tests: AU915 at 916.8 MHz, AS923 at 923.2 MHz, US915 at 903.9 MHz — all pass

## Key Files

### Modified
- `crates/maverick-core/src/protocol/lorawan_10x_class_a.rs` — `extend_fcnt`, `FcntError`, updated `validate_uplink`
- `crates/maverick-core/src/protocol/mod.rs` — re-export `FcntError`
- `crates/maverick-core/src/protocol/capability.rs` — `RejectFcntGapExceeded` variant
- `crates/maverick-adapter-radio-udp/src/gwmp.rs` — region fix + MIC/phy_without_mic extraction

## Self-Check: PASSED

- `extend_fcnt` with `MAX_FCNT_GAP = 16384` ✓
- `FcntError::Duplicate` and `GapExceeded` ✓
- `ProtocolDecision::RejectFcntGapExceeded` ✓
- `infer_region` AS923 before AU915 before US915 ✓
- `UplinkObservation` populated with `f_cnt:u16`, `wire_mic`, `phy_without_mic` ✓
- `cargo test -p maverick-adapter-radio-udp` → 11 passed ✓
- `cargo test -p maverick-core -- lorawan_10x` → all passed ✓
