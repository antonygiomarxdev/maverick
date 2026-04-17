---
phase: "03"
slug: "protocol-security"
status: complete
nyquist_compliant: false
wave_0_complete: true
created: "2026-04-17"
---

# Phase 03 — Validation Strategy

> Per-phase validation contract for protocol security (MIC, decryption, FCnt).

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `#[test]` / `#[tokio::test]` (built-in) |
| **Config file** | `Cargo.toml` per crate |
| **Quick run command** | `cargo test -p maverick-core --lib` |
| **Full suite command** | `cargo test --workspace` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p maverick-core --lib`
- **After every plan wave:** Run full suite
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 03-01-01 | 01 | 1 | PROT-01 / PROT-02 | — | MIC computation produces spec-compliant output | unit | `cargo test -p maverick-core --lib -- ingest_uplink::tests::mic_spec` | ✅ | ✅ green |
| 03-01-02 | 01 | 1 | PROT-03 | — | FRMPayload decryption produces spec-compliant output | unit | `cargo test -p maverick-core --lib -- ingest_uplink::tests::decrypt_spec` | ✅ | ✅ green |
| 03-01-03 | 01 | 1 | PROT-04 | — | FCnt 32-bit rollover detected correctly | unit | `cargo test -p maverick-core --lib -- lorawan_10x_class_a::tests::fcnt` | ✅ | ✅ green |
| 03-02-01 | 02 | 1 | PROT-05 | — | UDP adapter correctly extracts wire_mic and phy_without_mic | unit | `cargo test -p maverick-adapter-radio-udp --lib` | ✅ | ✅ green |
| 03-02-02 | 02 | 1 | PROT-01–06 | — | Full pipeline (UDP parse → MIC verify → decrypt → persist) works e2e | unit | `cargo test -p maverick-core --lib -- ingest_uplink::tests::ingest_e2e` | ❌ | ⚠️ MISSING |
| 03-02-03 | 02 | 1 | PROT-06 | — | SPI adapter contract documented for libloragw integration | unit | `cargo test -p maverick-adapter-radio-spi --lib --features spi` | ✅ | ⚠️ PARTIAL |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [x] Existing `cargo test` infrastructure covers MIC, decryption, FCnt, UDP parsing
- [x] No external test framework needed (Rust built-in)

*Existing infrastructure covers all phase requirements.*

---

## Gap Analysis

| Task | Requirement | Gap Type | Details |
|------|-------------|----------|---------|
| 03-02-02 | PROT-01–06 | MISSING | No end-to-end test exercising full `IngestUplink::execute()` pipeline with real MIC-carrying frame through UDP parse → MIC verify → decrypt → persist |
| 03-01-01 | PROT-01 / PROT-02 | PARTIAL | MIC spec vector tests use PLACEHOLDER expected values (marked in code); must verify against actual LoRaWAN 1.0.x spec Chapter 7 table |
| 03-01-02 | PROT-03 | PARTIAL | Decryption spec vector test uses PLACEHOLDER expected values; must verify against spec Chapter 7 table |
| 03-02-03 | PROT-06 | PARTIAL | SPI adapter contract documented; `spi_adapter_parsing_contract` test is `#[ignore]` pending libloragw integration |

### Classification

| Status | Count | Items |
|--------|-------|-------|
| **COVERED** | 3 | Tasks 03-01-01 (MIC), 03-01-02 (decrypt), 03-01-03 (FCnt), 03-02-01 (UDP MIC extraction) |
| **PARTIAL** | 3 | MIC/decrypt placeholders (need spec verification); SPI contract pending libloragw |
| **MISSING** | 1 | Task 03-02-02 — end-to-end pipeline test |

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Spec vector value verification | PROT-01, PROT-02, PROT-03 | LoRaWAN 1.0.x spec Chapter 7 table must be consulted to replace placeholder values | 1. Obtain LoRaWAN 1.0.x Specification Document Chapter 7 2. Extract test vectors for MIC and FRMPayload decryption 3. Replace placeholder expected values in `ingest_uplink.rs` tests 4. Re-run tests to confirm match |
| SPI libloragw integration | PROT-06 | Hardware integration requires actual loragw hardware; cannot test in CI | 1. Connect loragw hardware 2. Run `cargo test -p maverick-adapter-radio-spi --lib --features spi -- --ignored` 3. Verify `spi_adapter_parsing_contract` passes |

*1 automated gap remains: end-to-end pipeline test.*

---

## Validation Audit 2026-04-17

| Metric | Count |
|--------|-------|
| Gaps found | 4 |
| Resolved | 0 |
| Escalated | 0 |
| Partial | 3 |
| Missing | 1 |

---

## Validation Sign-Off

- [x] All tasks have automated verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [ ] `nyquist_compliant: true` — **blocked by 1 MISSING gap (03-02-02)**
- [ ] All automated gaps resolved

**Approval:** pending (2026-04-17)
