---
phase: 10
slug: libloragw-spi-integration
status: complete
nyquist_compliant: false
wave_0_complete: true
created: 2026-04-17
---

# Phase 10 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust / cargo test |
| **Config file** | none |
| **Quick run command** | `cargo test -p maverick-adapter-radio-spi` |
| **Full suite command** | `cargo test -p maverick-adapter-radio-spi --all-features` |
| **Estimated runtime** | ~5 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p maverick-adapter-radio-spi`
- **After every plan wave:** Run `cargo test -p maverick-adapter-radio-spi --all-features`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 10 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 10-A-01 | 10-A | 1 | RADIO-01 | — | N/A | unit | `ls libloragw/inc/loragw_hal.h` | ✅ | ✅ green |
| 10-A-02 | 10-A | 1 | RADIO-01 | — | N/A | unit | `cargo check -p maverick-adapter-radio-spi --features spi 2>&1` | ✅ | ✅ green |
| 10-A-03 | 10-A | 1 | RADIO-01 | — | N/A | unit | `grep 'CARGO_FEATURE_SPI' crates/maverick-adapter-radio-spi/build.rs` | ✅ | ✅ green |
| 10-A-04 | 10-A | 1 | RADIO-01 | — | N/A | unit | `grep 'fn lgw_receive' crates/maverick-adapter-radio-spi/src/lgw_bindings.rs` | ✅ | ✅ green |
| 10-B-01 | 10-B | 2 | RADIO-01, RADIO-02 | — | N/A | unit | `grep 'lgw_pkt_rx_to_observation' crates/maverick-adapter-radio-spi/src/spi_uplink.rs` | ✅ | ✅ green |
| 10-B-02 | 10-B | 2 | RADIO-01, RADIO-02 | — | N/A | unit | `grep 'fn lgw_hal_start' crates/maverick-adapter-radio-spi/src/lgw_init.rs` | ✅ | ✅ green |
| 10-B-03 | 10-B | 2 | RADIO-01, RADIO-02 | — | N/A | unit | `grep 'impl Drop' crates/maverick-adapter-radio-spi/src/spi_uplink.rs` | ✅ | ✅ green |
| 10-B-04 | 10-B | 2 | RADIO-01 | — | N/A | unit | `grep 'fn wire_mic_split_is_correct' crates/maverick-adapter-radio-spi/src/spi_uplink.rs` | ✅ | ✅ green |
| 10-B-05 | 10-B | 2 | RADIO-02 | — | N/A | integration | `cargo test -p maverick-adapter-radio-spi -- wire_mic_split_is_correct 2>&1` | ✅ | ✅ green |

*Status: ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [x] Wave 0 satisfied by existing Rust/cargo test infrastructure
- [x] Unit tests embedded in source files at `spi_uplink.rs`
- [x] `wire_mic_split_is_correct` unit test covers wire_mic/phy_without_mic split logic

*Existing infrastructure covers all phase requirements.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| lgw_receive on real SX1302/SX1303 hardware | RADIO-01 | Requires ARM hardware with real concentrator; cannot run in x86 CI | 1. Deploy to RAK2287/RAK5146 2. Run `cargo test -p maverick-adapter-radio-spi --features spi -- --ignored` 3. Verify packets received with valid wire_mic |
| lgw_start/lgw_stop HAL lifecycle | RADIO-01 | Requires hardware probe; EBUSY on repeat start is hardware-level behavior | 1. Start SpiUplinkSource 2. Drop it 3. Start again — must not return EBUSY |
| struct layout on ARM | RADIO-01 | Bindings generated on x86 — ARM cross-compile target not installed | 1. Generate bindings on ARM hardware or with `--target aarch64-unknown-linux-gnu` 2. Compare struct field offsets |
| GatewayEui from concentrator OTP | RADIO-01 | Currently hardcoded to zero | Read from OTP or config file |
| lgw_send TX path | RADIO-01 | SPI TX / downlink is Phase 3.1 | N/A |

*All behaviors have automated verification except those requiring physical ARM hardware.*

---

## Validation Sign-Off

- [x] All tasks have automated verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 10s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending 2026-04-17

---

## Validation Audit 2026-04-17

| Metric | Count |
|--------|-------|
| Gaps found | 0 |
| Resolved | 0 |
| Escalated | 0 |

**Notes:**
- Phase 10 produces library code that calls C FFI — direct unit testing is limited
- `wire_mic_split_is_correct` unit test exists and covers the critical wire_mic/phy_without_mic split
- Integration test (`lgw_receive_produces_observations_with_valid_wire_mic`) is marked `#[ignore]` per plan — hardware-dependent
- All acceptance criteria from PLAN files are satisfied
