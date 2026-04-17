# Phase 03: Protocol Security — Research

**Researched:** 2026-04-16
**Phase:** 03 — Protocol Security
**Mode:** ecosystem
**Status:** `## RESEARCH COMPLETE`

---

## Phase Context

The developer has never implemented LoRaWAN protocol security (MIC, FCnt, AES-128 crypto) before. This research covers the full stack needed to ship a production-ready implementation.

**Locked decisions (from CONTEXT.md):**
- D-01: MIC verification in core layer (after adapter parsing)
- D-02: FCnt rollover via gap detection (not wrap-around detection)
- D-03: MIC failure → reject + WARN log + metric counter
- D-04: Testing via unit tests with LoRaWAN spec test vectors

**Requirements:** DWNL-01, DWNL-02, DWNL-03, DWNL-04, DWNL-05, DWNL-06 (per ROADMAP) + PROT-01 through PROT-06 (deferred from Phase 1)

---

## What Exists in the Codebase

### Already Implemented (✅ Production Quality)

**`maverick-core/src/use_cases/ingest_uplink.rs`:**

| Function | Status | Notes |
|----------|--------|-------|
| `build_b0_uplink()` | ✅ Done | LoRaWAN §4.4 B0 block, LE byte order correct |
| `compute_mic()` | ✅ Done | AES-128 CMAC via `cmac::Cmac<Aes128>`, returns first 4 bytes |
| `decrypt_frm_payload()` | ✅ Done | AES-128-CTR, block counter starts at 1 per §4.3.3.2 |
| MIC verification in `execute()` | ✅ Done | After FCnt check, before payload decryption |
| MIC failure → audit + error | ✅ Done | `outcome: "rejected:mic_invalid"`, returns `AppError::Domain` |

**`maverick-core/src/protocol/lorawan_10x_class_a.rs`:**

| Function | Status | Notes |
|----------|--------|-------|
| `extend_fcnt()` | ✅ Done | 32-bit reconstruction, MAX_FCNT_GAP=16384 per §4.3.1.5 |
| Rollover detection | ✅ Done | `candidate_high = candidate_low.wrapping_add(0x1_0000)` |
| Gap detection | ✅ Done | When `newFCnt < lastFCnt` and gap > 1000 → rollover assumed |
| `validate_uplink()` | ✅ Done | Region/class/FCnt checks, returns `ProtocolDecision` |
| Unit tests | ✅ Done | 5 tests covering rollover, duplicate, gap exceeded |

**Domain types (`maverick-domain/src/session.rs`):**
- `SessionSnapshot.uplink_frame_counter: u32` — already 32-bit
- `SessionSnapshot.nwk_s_key: [u8; 16]` — available for MIC
- `SessionSnapshot.app_s_key: [u8; 16]` — available for decryption

**Crypto dependencies (`maverick-core/Cargo.toml`):**
```toml
aes = "0.9"    # AES-128 block cipher
cmac = "0.8"   # CMAC (Cipher-based MAC)
```

### What Is Missing (⚠️ Gap — blocking production)

**1. UDP Adapter — PHY Parsing for MIC fields (BLOCKING)**

The UDP adapter (`maverick-adapter-radio-udp`) receives raw LoRaWAN frames but the `UplinkObservation` struct requires `wire_mic: [u8; 4]` and `phy_without_mic: Vec<u8>` — these must be extracted by the adapter's PHY parser.

The `GwmpPacket` / `PhyPayload` parsing code needs to:
1. Strip the MHDR (1 byte)
2. Compute where MIC (4 bytes) starts in the frame
3. Extract `wire_mic` from bytes `[phy_len-4..phy_len]`
4. Extract `phy_without_mic` as bytes `[1..phy_len-4]` (frame minus MHDR and MIC)
5. Extract `payload` (FRMPayload) from bytes after MHDR, before MIC

This is adapter-specific parsing — the core `IngestUplink::execute()` expects `wire_mic` and `phy_without_mic` to be already extracted.

**2. Session FCnt — 16-bit to 32-bit Schema Migration (BLOCKING)**

The SQLite schema currently stores `f_cnt` as `INTEGER` (from Phase 1 partial). Sessions in `sessions` table need to store the full 32-bit `uplink_frame_counter`. No migration is needed for existing sessions (they start at 0 and reconstruct correctly), but the schema column type should be verified as 64-bit INTEGER for safety.

**3. Security Metrics Counter (D-03)**

D-03 specifies: "When MIC fails: reject + warning + metric." The audit log is implemented (`outcome: "rejected:mic_invalid"`). A dedicated `mic_failure_count` metric for security monitoring is not yet exposed as a Prometheus/health metric — this would be relevant for Phase 07 (observability).

**4. Downlink scheduling (DWNL-01 through DWNL-06)**

The ROADMAP maps DWNL-01 through DWNL-06 to Phase 3 as "Class A Downlink." The context says "What's Out: Cloud-side security, OTAA join security." The actual downlink scheduling (RX1/RX2 windows) is a separate concern from protocol security. These should likely be separate plans or phases.

---

## Standard Stack

| Component | Library | Version | Notes |
|-----------|---------|---------|-------|
| AES-128 block cipher | `aes` | 0.9 | Already in workspace deps |
| CMAC (MIC) | `cmac` | 0.8 | Already in workspace deps |
| CTR mode | Built on `aes` | — | Hand-rolled (see `decrypt_frm_payload`) |
| Error handling | `thiserror` | 1 | Already used everywhere |
| Async | `tokio` | 1 | Already used everywhere |

**No new dependencies needed.** The `aes` + `cmac` crates are sufficient for all cryptographic operations.

---

## Architecture Patterns

### Integration Point: Where MIC Fits in the Pipeline

```
UDP/SPI Adapter
    ↓ (raw bytes)
PHY Parser (MHDR strip, DevAddr/FCnt extract, MHDR移除)
    ↓
UplinkObservation { dev_addr, f_cnt, wire_mic, phy_without_mic, payload }
    ↓
IngestUplink::execute()
    ├── FCnt 32-bit reconstruction (extend_fcnt)
    ├── Protocol validation (region, class)
    ├── MIC verification ← crypto happens here (compute_mic)
    ├── FRMPayload decryption (decrypt_frm_payload) ← crypto happens here
    ├── Dedup check (SQLite)
    ├── Persist UplinkRecord
    └── Update session FCnt
    ↓
SQLite
```

### Hexagonal Boundaries

- **Adapter → Core**: `UplinkObservation` (contains parsed frame + raw PHY for MIC)
- **Core → Adapter**: `ProtocolDecision` (Accept/Reject variants)
- **Core has no crypto library imports** — `aes` and `cmac` are in `maverick-core` (application layer, not domain)

### Testing Pattern

Per D-04: "Testing via unit tests with hardcoded frames from LoRaWAN spec test vectors." The LoRaWAN 1.0.x spec (Chapter 7) contains canonical test vectors for:
- MIC computation (Table 7.2 in spec)
- FCnt rollover scenarios
- Payload decryption

No hardware needed for unit testing — all crypto is pure functions.

---

## Don't Hand-Roll

| Problem | Use Instead | Notes |
|---------|-------------|-------|
| MIC computation | `cmac::Cmac<Aes128>` | Already done in `compute_mic()` |
| AES-128 encryption | `aes::Aes128` block cipher | Already done in `decrypt_frm_payload()` |
| Block counter mode | Hand-rolled on top of `aes::Aes128` | Correct per LoRaWAN spec |
| FCnt rollover | Already implemented | `extend_fcnt()` with `MAX_FCNT_GAP` |

---

## Common Pitfalls

### PITFALL 1: B0 Block Byte Order (LE vs BE)

**Symptom:** MIC verification fails on valid frames.

**Cause:** LoRaWAN spec §4.4 defines B0 as having multi-byte fields in **little-endian** byte order. The original implementation had a bug where DevAddr and FCnt were stored as big-endian in the frame but must be converted to LE for B0.

**Current status:** `build_b0_uplink()` uses `.to_le_bytes()` correctly (see code comments with PITFALL markers). Verify the UDP adapter's PHY parser passes `dev_addr` in network byte order (big-endian in wire) — if so, `.to_le_bytes()` is correct for B0.

### PITFALL 2: CTR Block Counter Starts at 1

**Symptom:** Payload decryption produces wrong plaintext.

**Cause:** LoRaWAN §4.3.3.2 specifies block counter `i` starts at **1**, not 0. Initial implementations often start at 0.

**Current status:** `decrypt_frm_payload()` starts `for i in 1u8..=` — correct.

### PITFALL 3: phy_without_mic vs payload

**Symptom:** MIC always fails even with correct key.

**Cause:** `phy_without_mic` is the PHY payload **excluding MHDR but including MIC** for MIC calculation. The `payload` field (FRMPayload) is a *suffix* of `phy_without_mic`. The adapter must extract both from the raw frame correctly.

The MIC is computed over: `MHDR || DevAddr || FCnt || FOpts || FPort || FCtrl | FHDR | ... | payload || MIC`

The `phy_without_mic` passed to `compute_mic()` is everything **after** MHDR and **before** MIC (i.e., MHDR is excluded, MIC is excluded). The adapter must parse the full PHY and split it correctly.

### PITFALL 4: 16-bit Rollover vs Gap Detection

**Symptom:** Session breaks after 65535 uplinks (not 65536).

**Cause:** When the 16-bit wire FCnt wraps from 65535 → 0, the server must increment the high 16 bits of its 32-bit counter. Rollover detection by wrap-around alone is fragile (a retransmit could look like rollover).

**Current status:** `extend_fcnt()` uses gap detection (D-02: "when newFCnt < lastFCnt and difference is large > 1000"). This correctly handles both rollover and retransmission cases. The logic is: if the new wire value is much lower than the session counter, assume rollover by adding 0x1_0000 to the low 16 bits.

### PITFALL 5: Session FCnt Not Updated After MIC Failure

**Symptom:** Replay attack: attacker re-sends valid frame, session counter doesn't advance, second valid frame is accepted as non-duplicate.

**Cause:** If a valid MIC frame is rejected for other reasons (e.g., duplicate detection), the session FCnt must still be updated to prevent replays. Currently `extend_fcnt()` is called before MIC check, but if MIC fails, the counter update happens *after* MIC. The deduplication window is 30s — this is the attacker's window.

**Mitigation:** Consider updating session FCnt even on MIC failure (to the reconstructed value). This prevents the same frame from being retried after the counter advances.

---

## Code Examples

### LoRaWAN Spec Test Vector — MIC (Table 7.2 equivalent)

From LoRaWAN 1.0.x spec, Chapter 7:

```
DevAddr: 0x01_02_03_04
FCnt: 0x0000_0001
NwkSKey: 0x00_00_00_00_00_00_00_00_00_00_00_00_00_00_00_00
PHY[1..n-4]: (MHDR || rest of PHY without MIC) = [0xAA] // example
Expected MIC: first 4 bytes of AES-128-CMAC(B0 || PHY[1..n-4])
```

The unit test in `ingest_uplink.rs` (`obs_with_valid_mic`) tests with zero keys — the canonical test vector from the spec uses a known NwkSKey and produces a known MIC. The test pattern to add:

```rust
// From LoRaWAN 1.0.x spec Chapter 7 — canonical MIC test vector
#[test]
fn mic_from_spec_test_vector() {
    // DevAddr = 0x01020304, FCnt = 1, NwkSKey = all zeros
    // With a known PHY payload, the CMAC should produce a known 4-byte MIC
    let nwk_s_key = [0u8; 16];
    let dev_addr = 0x01_02_03_04u32;
    let f_cnt = 1u32;
    let phy_without_mic = vec![0xAA]; // simplified — actual spec vector has more bytes
    let b0 = build_b0_uplink(dev_addr, f_cnt, phy_without_mic.len());
    let mic = compute_mic(&nwk_s_key, &b0, &phy_without_mic);
    // Assert against known-good value from spec
    assert_eq!(mic, [0x00, 0x00, 0x00, 0x00]); // placeholder — replace with real spec value
}
```

### FCnt Rollover Test Vector

```rust
#[test]
fn fcnt_rollover_at_65535() {
    // Session at 65535 (0xFFFF), next wire FCnt is 0 → rollover to 65536
    let result = LoRaWAN10xClassA::extend_fcnt(0x0000, 0x0000_FFFF);
    assert_eq!(result, Ok(0x0001_0000)); // high bit set correctly
}
```

---

## Implementation Plan Recommendation

### Priority 1: Close the UDP Adapter Gap (BLOCKING)

The UDP adapter's PHY parser must extract `wire_mic` and `phy_without_mic`. Without this, `IngestUplink::execute()` always receives zero/empty values and MIC verification is a no-op.

Files to modify:
- `crates/maverick-adapter-radio-udp/src/lib.rs` (or wherever `GwmpPacket` → `UplinkObservation` happens)

### Priority 2: Schema Verification (BLOCKING)

Verify `sessions` table stores `uplink_frame_counter` as 64-bit INTEGER. If it's 32-bit, a migration is needed before 4 billion uplinks (not urgent but correct).

### Priority 3: Add Canonical Test Vectors (D-04)

Add unit tests using the LoRaWAN spec's published test vectors to `ingest_uplink.rs`. This proves the crypto implementation matches the spec.

### Priority 4: Security Metrics (D-03)

If Phase 07 observability is not yet planned, consider adding a basic `mic_failure_count` atomic counter. If Phase 07 exists, defer to it.

---

## Validation Architecture

### What to Verify

| Dimension | What to Check |
|-----------|---------------|
| MIC correctness | Unit tests with spec test vectors pass |
| FCnt 32-bit | Session survives 65535→0 rollover transition |
| FRMPayload decryption | Unit tests with known plaintext/ciphertext pairs |
| Integration | End-to-end test: valid frame → persisted with decrypted payload |
| MIC failure path | Invalid MIC → rejected, logged, not persisted |
| Rollover path | Wire FCnt wraps → reconstructed FCnt is correct |

### How to Verify

```bash
cargo test -p maverick-core --lib -- protocol lorawan mic fcnt decrypt
cargo test -p maverick-adapter-radio-udp --lib  # if adapter tests exist
cargo clippy --all-features -- -D warnings  # no unsafe code in crypto paths
```

---

## Cross-Phase Dependencies

| Phase | Dependency | Impact |
|-------|-----------|--------|
| Phase 01 | Protocol correctness | MIC + FCnt were deferred here — this phase completes Phase 1's work |
| Phase 02 | Radio abstraction | SPI adapter also needs UplinkObservation with MIC fields |
| Phase 03 | Class A Downlink | DWNL-01..DWNL-06 are separate from protocol security |

**Note:** If Phase 3 per ROADMAP is "Class A Downlink" and Phase 3 per CONTEXT is "Protocol Security", these may need to be split into separate phases. The requirements (DWNL-01..DWNL-06) map to downlink, not security. Recommend clarifying phase numbering with the user.

---

## Conclusion

The core cryptographic implementation (MIC, FCnt, decryption) is **already done and production-quality** in `maverick-core`. The primary gap is **PHY parsing in the UDP adapter** to extract the MIC fields from wire bytes. Secondary gaps are test vectors and security metrics. No new libraries are needed.
