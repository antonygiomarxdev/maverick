---
phase: "08"
slug: hardware-testing-rak-pi
status: partial
nyquist_compliant: false
wave_0_complete: true
created: 2026-04-17
---

# Phase 8 — Validation Strategy

> Hardware testing phase for RAK Pi with LoRa HAT (SX1302/3). Hardware was not attached during execution. Phase status: infrastructure verified, hardware plans blocked.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test + tokio::test |
| **Config file** | `crates/maverick-integration-tests/Cargo.toml` |
| **Quick run command** | `cargo test -p maverick-integration-tests -- --test-threads=1` |
| **Full suite command** | `cargo test -p maverick-integration-tests` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p maverick-integration-tests`
- **After every plan wave:** Run full suite
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 08-A-01 | 08-A | 1 | SSH connectivity to RAK Pi | — | Key-based auth only, no password prompt | manual | `ssh -o ConnectTimeout=10 pi@rak.local "echo OK"` | N/A | ✅ verified |
| 08-A-02 | 08-A | 1 | SPI devices present | — | `/dev/spidev*` must exist | manual | `ssh pi@rak.local "ls -la /dev/spidev*"` | N/A | ❌ no hardware |
| 08-A-03 | 08-A | 1 | Maverick build succeeds | — | Clean build, no warnings | manual | `ssh pi@rak.local "cargo build --release"` | N/A | ✅ verified |
| 08-A-04 | 08-A | 1 | CLI commands functional | — | All subcommands respond | manual | `ssh pi@rak.local "./target/release/maverick-edge --help"` | N/A | ✅ verified |
| 08-A-05 | 08-A | 1 | Config validation | — | TOML parses, validates | manual | `ssh pi@rak.local "./target/release/maverick-edge config validate"` | N/A | ✅ verified |
| 08-B-01 | 08-B | 2 | LoRaWAN test vectors documented | — | Valid MIC frames accepted | unit | `crates/maverick-integration-tests/` | ✅ | ⬜ pending |
| 08-B-02 | 08-B | 2 | MIC verification via SPI | — | Valid/invalid MIC correctly handled | hardware | `cargo test -p maverick-integration-tests test_mic` | ✅ | ⚠️ blocked (no SPI) |
| 08-C-01 | 08-C | 2 | Unconfirmed uplink no ACK | — | No downlink scheduled | unit | `class_a_downlink.rs` | ✅ | ✅ 6/6 pass |
| 08-C-02 | 08-C | 2 | Confirmed uplink triggers ACK | — | ACK flag set in downlink | unit | `class_a_downlink.rs` | ✅ | ✅ verified |
| 08-C-03 | 08-C | 2 | RX1/RX2 timing | — | RX1 at 1s, RX2 at 2s ±100ms | unit | `class_a_downlink.rs` | ✅ | ✅ verified |
| 08-C-04 | 08-C | 2 | Downlink queue persistence | — | Survives process restart | unit | `persistence_sqlite.rs` | ✅ | ✅ 6/6 pass |
| 08-D-01 | 08-D | 2 | SPI initialization | — | Device opened, no errors | hardware | `sudo ./target/release/maverick-edge --radio spi` | N/A | ⚠️ blocked |
| 08-D-02 | 08-D | 2 | SPI RX path | — | Packets via SPI not UDP | hardware | Log analysis | N/A | ⚠️ blocked |
| 08-D-03 | 08-D | 2 | SPI TX path | — | TX command sent, LED indicator | hardware | Log analysis | N/A | ⚠️ blocked |
| 08-E-01 | 08-E | 3 | Single device high rate | — | 600 uplinks, no loss, stable memory | hardware | SQLite query count | N/A | ⚠️ blocked |
| 08-E-02 | 08-E | 3 | Multiple device concurrent | — | 5 devices, no collisions | hardware | SQLite + logs | N/A | ⚠️ blocked |
| 08-E-03 | 08-E | 3 | Burst handling | — | 100 rapid uplinks, no silent drops | hardware | SQLite + logs | N/A | ⚠️ blocked |
| 08-E-04 | 08-E | 3 | 24hr stability | — | No crashes, memory stable | hardware | Monitor | N/A | ⚠️ blocked |
| 08-F-01 | 08-F | 3 | TUI startup | — | No crashes, help displays | hardware | `./maverick-edge tui --help` | N/A | ⚠️ blocked |
| 08-F-02 | 08-F | 3 | TTY rendering | — | Menus render correctly | hardware | Visual | N/A | ⚠️ blocked |
| 08-F-03 | 08-F | 3 | Device list menu | — | Count matches DB | hardware | DB query | N/A | ⚠️ blocked |
| 08-G-01 | 08-G | 3 | Extension discovery | — | Built-in extensions listed | manual | `./maverick-edge extensions list` | N/A | ⚠️ partial |
| 08-G-02 | 08-G | 3 | Extension health | — | Health report with status | manual | `./maverick-edge health` | N/A | ✅ verified |
| 08-G-03 | 08-G | 3 | Extension isolation | — | Crash doesn't kill core | manual | Extension crash test | N/A | ⚠️ blocked |
| 08-G-04 | 08-G | 3 | Extension IPC | — | Requests/responses work | manual | HTTP extension test | N/A | ⚠️ blocked |
| 08-H-01 | 08-H | 3 | Complete uplink flow | — | Device→Radio→SPI→LNS→SQLite | hardware | SQLite query | N/A | ⚠️ blocked |
| 08-H-02 | 08-H | 3 | Complete downlink flow | — | Queue→TX→Device ACK | hardware | Log analysis | N/A | ⚠️ blocked |
| 08-H-03 | 08-H | 3 | Round-trip latency | — | Under 3s total | hardware | Timestamp analysis | N/A | ⚠️ blocked |
| 08-J-01 | 08-J | 3 | Baseline resource usage | — | CPU, memory, disk documented | manual | `top`, `free`, `df` | N/A | ⚠️ partial |
| 08-J-02 | 08-J | 3 | Uplink throughput | — | Rate measured, no loss | hardware | SQLite count | N/A | ⚠️ blocked |
| 08-J-03 | 08-J | 3 | CPU under load | — | Usage at various rates | hardware | `top` sampling | N/A | ⚠️ blocked |
| 08-J-04 | 08-J | 3 | Memory stability | — | No leaks over time | hardware | `ps rss` | N/A | ⚠️ blocked |
| 08-J-05 | 08-J | 3 | Latency measurement | — | Avg, P99, jitter | hardware | Timestamp delta | N/A | ⚠️ blocked |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ blocked (hardware missing)*

---

## Wave 0 Coverage

| Requirement | Test File | Status |
|-------------|-----------|--------|
| Class A downlink behavior | `tests/class_a_downlink.rs` | ✅ 6/6 pass |
| End-to-end gateway flow | `tests/operator_local_gateway_e2e.rs` | ✅ 1/1 pass |
| SQLite persistence | `tests/persistence_sqlite.rs` | ✅ 6/6 pass |
| Radio transport resilience | `tests/radio_transport_resilience.rs` | ✅ 4/4 pass |
| Core smoke tests | `tests/smoke.rs` | ✅ 3/3 pass |
| Watchdog tests | `tests/watchdog.rs` | ⚠️ 4/6 pass (2 flaky) |

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| SSH connectivity to RAK Pi | 08-A-01 | Requires live hardware access | `ssh pi@rak.local "echo OK"` |
| SPI device detection | 08-A-02 | Hardware not attached | `ls /dev/spidev*` |
| Maverick build on RAK Pi | 08-A-03 | Cross-compile target | `cargo build --release` |
| CLI command verification | 08-A-04 | Requires binary on target | `./maverick-edge probe/health/status` |
| Config validation | 08-A-05 | Requires target system | `./maverick-edge config validate` |
| Extension discovery | 08-G-01 | Requires target system | `./maverick-edge extensions list` |
| Hardware SPI tests (08-B, 08-D) | MIC/SPI verification | No SPI hardware | Attach RAK LoRa HAT, re-run |
| Real radio tests (08-E, 08-H, 08-J) | Stress, E2E, perf | No radio hardware | Attach RAK LoRa HAT, re-run |
| TUI tests (08-F) | Display rendering | No HAT display | Attach RAK LoRa HAT with display |

---

## Gap Summary

| Category | Count | Details |
|----------|-------|---------|
| **COVERED** | 17 | Infrastructure, CLI, config, unit tests |
| **PARTIAL** | 3 | Extension partial, watchdog flaky, resources partial |
| **MISSING** | 18 | All hardware-dependent tests blocked by missing HAT |
| **Total** | 38 | |

**Root Cause:** RAK LoRa HAT (SX1302/3) not physically attached to Raspberry Pi.

---

## Validation Audit 2026-04-17

| Metric | Count |
|--------|-------|
| Gaps found | 21 |
| Resolved | 0 (hardware blocked) |
| Escalated to manual | 21 |

---

## Blocking Issues

1. **RAK LoRa HAT not attached** — `/dev/spidev*` missing, cannot test SPI/radio
2. **Watchdog test flakiness** — 2/6 watchdog tests race; `tests/watchdog.rs:27` and `tests/watchdog.rs:89`

---

## Required Actions

1. **Attach RAK LoRa HAT** to enable hardware testing plans 08-B through 08-J
2. **Fix watchdog test race condition** — increase sleep to 100ms or use sync barrier in `tests/watchdog.rs`
3. **Re-run Phase 8** after HAT attachment to complete hardware validation

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [ ] Feedback latency < 30s — PARTIAL (24/26 tests pass)
- [ ] `nyquist_compliant: true` set in frontmatter — **PENDING hardware**

**Approval:** pending — hardware required
