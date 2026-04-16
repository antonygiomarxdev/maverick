---
phase: 2
slug: radio-abstraction-spi
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-04-16
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
| **Full suite command** | `cargo test --workspace` |
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
| 2-A-01 | A | 1 | RELI-05 | — | N/A | unit | `cargo test -p maverick-core -- ports::uplink_source` | ❌ W0 | ⬜ pending |
| 2-A-02 | A | 1 | RELI-05 | — | N/A | unit | `cargo test -p maverick-adapter-radio-udp` | ❌ W0 | ⬜ pending |
| 2-B-01 | B | 1 | RADIO-03 | T-2-01 | spi_path validated as /dev/spidev* | unit | `cargo test -p maverick-core -- lns_config` | ❌ W0 | ⬜ pending |
| 2-C-01 | C | 2 | RADIO-01/02 | — | N/A | compilation | `cargo build -p maverick-adapter-radio-spi --features spi` (ARM) | ❌ W0 | ⬜ pending |
| 2-D-01 | D | 2 | RADIO-01 | — | N/A | integration | `cargo test -p maverick-integration-tests --test operator_local_gateway_e2e` | ✅ | ⬜ pending |
| 2-E-01 | E | 3 | RADIO-04/CORE-04 | — | N/A | manual | `toml parse hardware-registry.toml` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/maverick-core/src/ports/uplink_source.rs` — `UplinkSource` trait
- [ ] `crates/maverick-adapter-radio-udp/src/uplink_source.rs` — `GwmpUdpUplinkSource` impl stub + unit tests
- [ ] `crates/maverick-adapter-radio-spi/` — new crate with `Cargo.toml`, `build.rs`, `lib.rs` skeleton
- [ ] `crates/maverick-core/src/lns_config.rs` — `RadioConfig` / `RadioBackend` structs + deserialization unit tests
- [ ] `hardware-registry.toml` — initial file with RAK Pi HAT entry

*Wave 0 must be complete before any wave that attempts `cargo build --features spi`.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Maverick reads uplinks directly from RAK Pi HAT via SPI without packet forwarder | RADIO-02 | Requires physical Raspberry Pi + RAK LoRa HAT hardware | Boot Pi, install maverick-edge with `spi` feature, set `[radio] backend = "spi"`, confirm uplinks appear in SQLite |
| hardware-registry.toml contains RAK entry with correct fields | RADIO-04 | File content validation | `cat hardware-registry.toml` and verify: board_name="RAK2287", arch="aarch64/armv7", concentrator_model="sx1302" |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 60s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
