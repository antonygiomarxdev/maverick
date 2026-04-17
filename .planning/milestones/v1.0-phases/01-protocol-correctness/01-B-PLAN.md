---
phase: 01-protocol-correctness
plan: B
type: execute
wave: 2
depends_on:
  - 01-A
files_modified:
  - crates/maverick-core/src/protocol/lorawan_10x_class_a.rs
  - crates/maverick-adapter-radio-udp/src/gwmp.rs
autonomous: true
requirements:
  - PROT-02
  - PROT-05

must_haves:
  truths:
    - "extend_fcnt(wire_u16:u16, session_fcnt:u32) -> Result<u32, FcntError> exists as pub fn on LoRaWAN10xClassA"
    - "validate_uplink uses the reconstructed 32-bit FCnt from extend_fcnt, not the raw u16"
    - "MAX_FCNT_GAP = 16384 constant is defined and enforced"
    - "FcntError enum has Duplicate and GapExceeded variants"
    - "ProtocolDecision has RejectFcntGapExceeded variant alongside existing RejectDuplicateFrameCounter"
    - "infer_region in gwmp.rs places AS923 and AU915 match arms BEFORE US915"
    - "existing gwmp tests still pass"
  artifacts:
    - path: "crates/maverick-core/src/protocol/lorawan_10x_class_a.rs"
      provides: "extend_fcnt helper + updated validate_uplink + FcntError type"
      contains: "MAX_FCNT_GAP"
    - path: "crates/maverick-adapter-radio-udp/src/gwmp.rs"
      provides: "corrected infer_region frequency ordering"
      contains: "As923"
  key_links:
    - from: "crates/maverick-core/src/use_cases/ingest_uplink.rs"
      to: "crates/maverick-core/src/protocol/lorawan_10x_class_a.rs"
      via: "IngestUplink::execute calls extend_fcnt before validate_uplink"
      pattern: "extend_fcnt"
    - from: "crates/maverick-adapter-radio-udp/src/gwmp.rs"
      to: "crates/maverick-core/src/ports/radio_transport.rs"
      via: "rxpk_to_observation sets UplinkObservation.f_cnt:u16 from parsed wire bytes"
      pattern: "f_cnt:"
---

<objective>
Implement 32-bit FCnt reconstruction with MAX_FCNT_GAP enforcement, fix the region inference bug, and update the GWMP parser's f_cnt field to match the new u16 type.

Purpose: PROT-02 requires that a device sending more than 65535 uplinks keeps working. The current code casts the 16-bit wire value directly to u32 and compares it as-is — this silently breaks any session past frame 65535. Additionally, PROT-05 requires AU915 and AS923 frames to be identified correctly; the current `infer_region` shadowing means AU915 frequencies fall into US915.

Output: `extend_fcnt` public function on `LoRaWAN10xClassA`; `validate_uplink` uses the reconstructed counter; `infer_region` has correct arm ordering; GWMP parser constructs `UplinkObservation` with the u16 `f_cnt` field and populates `wire_mic` and `phy_without_mic`.
</objective>

<execution_context>
@/root/.claude/get-shit-done/workflows/execute-plan.md
@/root/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/phases/01-protocol-correctness/1-CONTEXT.md
@.planning/phases/01-protocol-correctness/01-RESEARCH.md
@.planning/phases/01-protocol-correctness/01-PATTERNS.md
@.planning/phases/01-protocol-correctness/01-A-SUMMARY.md
</context>

<tasks>

<task type="auto">
  <name>Task B-1: FCnt 32-bit extension in protocol module</name>
  <files>
    crates/maverick-core/src/protocol/lorawan_10x_class_a.rs
  </files>
  <read_first>
    - crates/maverick-core/src/protocol/lorawan_10x_class_a.rs
    - crates/maverick-core/src/protocol/mod.rs
  </read_first>
  <action>
**lorawan_10x_class_a.rs** — Four changes:

1. Add `FcntError` enum before the `LoRaWAN10xClassA` struct:
```rust
/// Error returned by `extend_fcnt` when a frame must be rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FcntError {
    /// Wire FCnt falls within the gap window below session counter — frame is a duplicate or replay.
    Duplicate,
    /// Gap between reconstructed FCnt and session counter exceeds MAX_FCNT_GAP (16384).
    GapExceeded,
}
```

2. Add `RejectFcntGapExceeded` to `ProtocolDecision` in the protocol module (check where ProtocolDecision is defined — likely `crates/maverick-core/src/protocol/mod.rs`). Add the new variant alongside the existing rejection variants.

3. Add `extend_fcnt` as a `pub fn` on the `LoRaWAN10xClassA` impl block (same impl block as `region_supported`, before the `ProtocolCapability` impl):
```rust
impl LoRaWAN10xClassA {
    fn region_supported(region: RegionId) -> bool { ... }

    /// LoRaWAN 1.0.x §4.3.1.5 — extend 16-bit wire FCnt to 32-bit server counter.
    ///
    /// Returns Ok(reconstructed_fcnt) if the frame should be processed.
    /// Returns Err(FcntError::Duplicate) if the frame counter is within the replay window.
    /// Returns Err(FcntError::GapExceeded) if the gap exceeds MAX_FCNT_GAP.
    pub fn extend_fcnt(wire_u16: u16, session_fcnt: u32) -> Result<u32, FcntError> {
        const MAX_FCNT_GAP: u32 = 16384; // LoRaWAN spec §4.3.1.5

        let candidate_low = (session_fcnt & 0xFFFF_0000) | u32::from(wire_u16);
        let candidate_high = candidate_low.wrapping_add(0x1_0000);

        if candidate_low > session_fcnt {
            // Normal forward progress — no rollover needed
            Ok(candidate_low)
        } else if session_fcnt.wrapping_sub(candidate_low) <= MAX_FCNT_GAP {
            // candidate_low <= session_fcnt AND within gap window → duplicate/replay
            Err(FcntError::Duplicate)
        } else if candidate_high.wrapping_sub(session_fcnt) <= MAX_FCNT_GAP {
            // 16-bit rollover: low candidate is in the past but high candidate is close enough
            Ok(candidate_high)
        } else {
            // Gap too large in both directions
            Err(FcntError::GapExceeded)
        }
    }
}
```

4. Update `validate_uplink` to call `extend_fcnt` and use the reconstructed counter. The method now receives `obs.f_cnt` as `u16` (from Plan A). Replace the current FCnt comparison block (lines 43-47) with:
```rust
// FCnt 32-bit reconstruction (D-07, D-08)
match Self::extend_fcnt(obs.f_cnt, session.uplink_frame_counter) {
    Err(FcntError::Duplicate) => return Ok(ProtocolDecision::RejectDuplicateFrameCounter),
    Err(FcntError::GapExceeded) => return Ok(ProtocolDecision::RejectFcntGapExceeded),
    Ok(_reconstructed) => {} // Accept; IngestUplink::execute will use the reconstructed value
}
```

Note: `validate_uplink` returns `ProtocolDecision` — it does not need to return the reconstructed value. `IngestUplink::execute` will call `extend_fcnt` itself to get the u32 for persistence (Plan C will wire this). The protocol module just gates acceptance/rejection.

5. Update the existing tests in `#[cfg(test)]` module:
- `sample_observation(fc: u32)` must change to `sample_observation(fc: u16)` and update the `f_cnt` field
- `sample_observation` callers pass `u16` literals: `sample_observation(6u16)`, `sample_observation(10u16)`, etc.
- The `UplinkObservation` struct literal in `sample_observation` now also requires `wire_mic: [0u8; 4]` and `phy_without_mic: vec![]` (zero-filled; not used in these tests)
- Add tests for `extend_fcnt`:
  ```rust
  #[test]
  fn extend_fcnt_no_rollover() {
      assert_eq!(LoRaWAN10xClassA::extend_fcnt(0x0010, 0x0000_0005), Ok(0x0000_0010));
  }
  #[test]
  fn extend_fcnt_rollover_at_16bit_boundary() {
      assert_eq!(LoRaWAN10xClassA::extend_fcnt(0x0001, 0x0000_FFFE), Ok(0x0001_0001));
  }
  #[test]
  fn extend_fcnt_duplicate_rejected() {
      assert_eq!(LoRaWAN10xClassA::extend_fcnt(0x0005, 0x0000_0010), Err(FcntError::Duplicate));
  }
  #[test]
  fn extend_fcnt_gap_exceeded_rejected() {
      // Gap of 20000 > MAX_FCNT_GAP 16384
      assert_eq!(LoRaWAN10xClassA::extend_fcnt(0x9999, 0x0001_0000), Err(FcntError::GapExceeded));
  }
  ```
  </action>
  <verify>
    <automated>cargo test -p maverick-core -- lorawan_10x extend_fcnt 2>&1 | tail -20</automated>
  </verify>
  <done>
    - `extend_fcnt` is `pub fn` on `LoRaWAN10xClassA`, takes `(u16, u32)`, returns `Result<u32, FcntError>`
    - `FcntError` enum has `Duplicate` and `GapExceeded` variants
    - `ProtocolDecision::RejectFcntGapExceeded` variant exists
    - `validate_uplink` calls `extend_fcnt` and returns `RejectDuplicateFrameCounter` or `RejectFcntGapExceeded` on error
    - All 4 `extend_fcnt` unit tests pass
    - Existing `accepts_incrementing_fcnt` and `rejects_duplicate_fcnt` tests pass (updated for u16 f_cnt)
    - `cargo test -p maverick-core -- lorawan_10x` passes
  </done>
</task>

<task type="auto">
  <name>Task B-2: Fix region inference and update GWMP parser for new UplinkObservation fields</name>
  <files>
    crates/maverick-adapter-radio-udp/src/gwmp.rs
  </files>
  <read_first>
    - crates/maverick-adapter-radio-udp/src/gwmp.rs
  </read_first>
  <action>
**gwmp.rs** — Three changes:

1. Fix `infer_region`. The current implementation has US915 (902–928 MHz) shadowing AU915 (915–928 MHz) and AS923 (920–923.5 MHz). Replace the entire `infer_region` function body with arms ordered most-specific-first:
```rust
fn infer_region(freq_mhz: Option<f64>) -> RegionId {
    match freq_mhz {
        Some(v) if (923.0..=923.5).contains(&v) => RegionId::As923, // AS923 first — most specific (subset of AU915)
        Some(v) if (915.0..=928.0).contains(&v) => RegionId::Au915, // AU915 before US915
        Some(v) if (902.0..915.0).contains(&v) => RegionId::Us915,  // US915 upper < 915
        Some(v) if (863.0..=870.0).contains(&v) => RegionId::Eu868,
        Some(v) if (433.0..=434.8).contains(&v) => RegionId::Eu433,
        _ => RegionId::Eu868,
    }
}
```
Note: US915 range changed to `902.0..915.0` (exclusive upper, not 928) to avoid overlap with AU915.

2. Update `parse_lorawan_payload` to also return `wire_mic` and `phy_without_mic`. Change the return type from `AppResult<(DevAddr, u32, u8, Vec<u8>)>` to `AppResult<(DevAddr, u16, u8, Vec<u8>, [u8; 4], Vec<u8>)>` (dev_addr, f_cnt_u16, f_port, payload, wire_mic, phy_without_mic).

The MIC is the last 4 bytes of the raw PHY payload. `phy_without_mic` is everything before those 4 bytes (MHDR + FHDR + FPort + FRMPayload). Update the existing MIC stripping logic:
```rust
let mic_len = 4;
if raw.len() < mic_len {
    return Err(AppError::InvalidInput("lorawan payload too short for MIC".to_string()));
}
let mut wire_mic = [0u8; 4];
wire_mic.copy_from_slice(&raw[raw.len() - mic_len..]);
let phy_without_mic = raw[..raw.len() - mic_len].to_vec();

// f_cnt is already u16 from the wire (per D-09)
let fcnt_u16 = u16::from_le_bytes([raw[LORAWAN_FHDR_FCNT_START], raw[LORAWAN_FHDR_FCNT_END - 1]]);

// payload = FRMPayload (excluding MIC at end)
let frm_payload_end = raw.len() - mic_len;
let payload = if frm_payload_start < frm_payload_end {
    raw[frm_payload_start..frm_payload_end].to_vec()
} else {
    vec![]
};

Ok((dev_addr, fcnt_u16, f_port, payload, wire_mic, phy_without_mic))
```

Remove the old `fcnt as u32` cast — the value is now returned as `u16`.

3. Update `rxpk_to_observation` to destructure the new tuple and populate all `UplinkObservation` fields:
```rust
fn rxpk_to_observation(gateway_eui: GatewayEui, rx: Rxpk) -> AppResult<UplinkObservation> {
    let decoded = B64
        .decode(rx.data.as_bytes())
        .map_err(|e| AppError::InvalidInput(format!("gwmp rxpk data base64: {e}")))?;
    let (dev_addr, f_cnt, f_port, payload, wire_mic, phy_without_mic) =
        parse_lorawan_payload(&decoded)?;
    Ok(UplinkObservation {
        gateway_eui,
        dev_addr,
        region: infer_region(rx.freq),
        f_cnt,           // u16 — no cast needed
        f_port,
        payload,
        rssi: rx.rssi,
        snr: rx.lsnr,
        wire_mic,
        phy_without_mic,
    })
}
```

Add region inference tests in `#[cfg(test)]`:
```rust
#[test]
fn infer_region_au915_not_shadowed_by_us915() {
    // 916.8 MHz is AU915 uplink channel 8
    let gw = GatewayEui(Eui64([1; 8]));
    let body = format!(r#"{{"rxpk":[{{"freq":916.8,"rssi":-70,"lsnr":6.0,"data":"QAECAwQEAAEByv66vg=="}}]}}"#);
    let batch = parse_push_data_json(gw, 2, &body).expect("batch");
    assert_eq!(batch.observations[0].region, RegionId::Au915);
}

#[test]
fn infer_region_as923_identified() {
    let gw = GatewayEui(Eui64([1; 8]));
    let body = format!(r#"{{"rxpk":[{{"freq":923.2,"rssi":-70,"lsnr":6.0,"data":"QAECAwQEAAEByv66vg=="}}]}}"#);
    let batch = parse_push_data_json(gw, 2, &body).expect("batch");
    assert_eq!(batch.observations[0].region, RegionId::As923);
}

#[test]
fn infer_region_us915_below_915() {
    // 903.9 MHz is US915 channel 7
    let gw = GatewayEui(Eui64([1; 8]));
    let body = format!(r#"{{"rxpk":[{{"freq":903.9,"rssi":-70,"lsnr":6.0,"data":"QAECAwQEAAEByv66vg=="}}]}}"#);
    let batch = parse_push_data_json(gw, 2, &body).expect("batch");
    assert_eq!(batch.observations[0].region, RegionId::Us915);
}
```
  </action>
  <verify>
    <automated>cargo test -p maverick-adapter-radio-udp 2>&1 | tail -20</automated>
  </verify>
  <done>
    - `infer_region` AS923 and AU915 arms appear before US915 arm
    - `infer_region(Some(916.8))` returns `RegionId::Au915`
    - `infer_region(Some(923.2))` returns `RegionId::As923`
    - `infer_region(Some(903.9))` returns `RegionId::Us915`
    - `UplinkObservation` is populated with `f_cnt:u16`, `wire_mic:[u8;4]`, `phy_without_mic:Vec<u8>`
    - All existing gwmp tests pass plus new region tests
    - `cargo test -p maverick-adapter-radio-udp` passes
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| Wire FCnt (u16) → session counter (u32) | Untrusted 16-bit value from UDP packet; reconstruction must not allow rollover abuse |
| GWMP freq field → RegionId | Floating-point frequency from external gateway; must map deterministically |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-01-B-01 | Repudiation | extend_fcnt replay window | mitigate | MAX_FCNT_GAP = 16384 per LoRaWAN spec §4.3.1.5; frames outside this window return GapExceeded and are rejected |
| T-01-B-02 | Spoofing | infer_region frequency overlap | mitigate | Arm ordering fix ensures AS923 and AU915 are matched before US915; no overlap in effective ranges |
| T-01-B-03 | Tampering | wire FCnt rollover arithmetic | mitigate | Use `wrapping_sub` and `wrapping_add` to avoid u32 overflow UB; tested with boundary values |
</threat_model>

<verification>
After both tasks complete:

```bash
cargo test -p maverick-core -- lorawan_10x 2>&1 | tail -20
cargo test -p maverick-adapter-radio-udp 2>&1 | tail -20
grep -n "extend_fcnt" crates/maverick-core/src/protocol/lorawan_10x_class_a.rs
grep -n "MAX_FCNT_GAP" crates/maverick-core/src/protocol/lorawan_10x_class_a.rs
grep -n "RejectFcntGapExceeded" crates/maverick-core/src/protocol/mod.rs
grep -n "infer_region" crates/maverick-adapter-radio-udp/src/gwmp.rs
grep -n "As923" crates/maverick-adapter-radio-udp/src/gwmp.rs
```
</verification>

<success_criteria>
- `extend_fcnt` exists with correct rollover logic and MAX_FCNT_GAP = 16384 — cargo test passes
- `ProtocolDecision::RejectFcntGapExceeded` variant exists
- `validate_uplink` uses extend_fcnt (not raw u16 comparison) — grep-verifiable
- `infer_region` AS923 match arm is first, AU915 is second, US915 is third — grep-verifiable
- `UplinkObservation` constructed with `f_cnt:u16`, `wire_mic`, `phy_without_mic` — grep-verifiable
- All tests in `maverick-core` and `maverick-adapter-radio-udp` pass
</success_criteria>

<output>
After completion, create `.planning/phases/01-protocol-correctness/01-B-SUMMARY.md`
</output>
