---
phase: 2
slug: radio-abstraction-spi
status: validated
nyquist_compliant: true
wave_0_complete: true
created: 2026-04-16
last_audited: 2026-04-17
---

# Phase 2 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (native) |
| **Config file** | none — workspace `Cargo.toml` |
| **Quick run command** | `cargo test --workspace` |
| **Full suite command** | `cargo test --workspace && cargo clippy --all-targets -- -D warnings` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --workspace`
- **After every plan wave:** Run `cargo test --workspace` + `cargo clippy --all-targets -- -D warnings`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 60 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 2-A-01 | A | 1 | RELI-05 | — | N/A | unit | `cargo test -p maverick-core -- lns_config` | ✅ | ✅ green |
| 2-A-02 | A | 1 | RELI-05 | — | N/A | unit | `cargo test -p maverick-adapter-radio-udp` | ✅ | ✅ green |
| 2-B-01 | B | 1 | RADIO-03 | T-2-01 | spi_path validated as /dev/spidev* | unit | `cargo test -p maverick-core -- lns_config` | ✅ | ✅ green |
| 2-C-01 | C | 2 | RADIO-01/02 | — | N/A | compilation | `cargo build -p maverick-adapter-radio-spi --features spi` | ✅ | ⚠️ partial |
| 2-D-01 | D | 2 | RADIO-01 | — | N/A | integration | `cargo test -p maverick-integration-tests --test operator_local_gateway_e2e` | ✅ | ✅ green |
| 2-E-01 | E | 3 | RADIO-04/CORE-04 | — | N/A | manual | File exists + content check | ✅ | ✅ manual |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ partial/flaky*

---

## Coverage Analysis

| Task | Requirement | Coverage | Notes |
|------|-------------|----------|-------|
| 2-A-01 | RELI-05 | PARTIAL | `UplinkSource` trait exists; no dedicated unit tests for trait itself (covered by integration) |
| 2-A-02 | RELI-05 | COVERED | 14 UDP adapter tests pass |
| 2-B-01 | RADIO-03 | COVERED | 7 lns_config tests pass including SPI validation |
| 2-C-01 | RADIO-01/02 | PARTIAL | SPI crate builds; libloragw RX is placeholder (not yet integrated) |
| 2-D-01 | RADIO-01 | COVERED | e2e integration test passes |
| 2-E-01 | RADIO-04/CORE-04 | MANUAL | File exists with correct schema |

---

## Wave 0 Requirements

All Wave 0 artifacts delivered:

- [x] `crates/maverick-core/src/ports/uplink_source.rs` — `UplinkSource` trait
- [x] `crates/maverick-adapter-radio-udp/src/gwmp_udp_uplink_source.rs` — `GwmpUdpUplinkSource` impl
- [x] `crates/maverick-adapter-radio-spi/` — new crate with `Cargo.toml`, `build.rs`, `lib.rs`
- [x] `crates/maverick-core/src/lns_config.rs` — `RadioConfig` / `RadioBackend` structs + deserialization unit tests
- [x] `docs/hardware-registry.toml` — initial file with RAK Pi HAT entry

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Maverick reads uplinks directly from RAK Pi HAT via SPI without packet forwarder | RADIO-02 | Requires physical Raspberry Pi + RAK LoRa HAT hardware | Boot Pi, install maverick-edge with `spi` feature, set `[radio] backend = "spi"`, confirm uplinks appear in SQLite |
| hardware-registry.toml contains RAK entry with correct fields | RADIO-04 | File content validation | `cat docs/hardware-registry.toml` and verify: board_name="RAK2287", arch="aarch64/armv7", concentrator_model="sx1302" |

---

## Validation Audit 2026-04-17

| Metric | Count |
|--------|-------|
| Gaps found | 2 |
| Resolved | 0 |
| Escalated | 0 |
| Manual-only | 2 |

**Partial items:**
- Task 2-A-01: `UplinkSource` trait lacks dedicated unit tests; covered by integration tests
- Task 2-C-01: SPI adapter is placeholder (libloragw RX not integrated); blocked on Phase 8 (Hardware Testing)

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 60s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** pending (manual verifications require hardware)

**Note:** Phase is validated as PARTIAL — SPI libloragw integration deferred to Phase 8. Automated tests pass for implemented functionality.
