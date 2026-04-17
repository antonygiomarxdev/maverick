---
phase: 09
slug: hardware-auto-detection-spi-enable
status: complete
nyquist_compliant: true
wave_0_complete: true
created: 2026-04-17
---

# Phase 09 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust / cargo test |
| **Config file** | none — Cargo.toml workspace |
| **Quick run command** | `cargo test --package maverick-runtime-edge` |
| **Full suite command** | `cargo test --workspace` |
| **Estimated runtime** | ~45 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --package maverick-runtime-edge`
- **After every plan wave:** Run `cargo test --workspace`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 45 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 09-A-01 | A | 1 | CORE-03, CORE-04, RADIO-01, RADIO-02 | — | Best-effort detection never fails startup | unit | `cargo test --package maverick-runtime-edge -- runtime_capabilities` | ✅ | ✅ green |
| 09-B-01 | B | 1 | CORE-03, RADIO-03 | — | Auto mode validates without spi_path | unit | `cargo test --package maverick-core` | ✅ | ✅ green |
| 09-B-02 | B | 1 | CORE-03, RADIO-03 | — | AutoSpi/AutoUdp variants resolve correctly | unit | `cargo test --package maverick-runtime-edge` | ✅ | ✅ green |
| 09-C-01 | C | 2 | CORE-03 | — | SPI guidance shown only when applicable | unit | `cargo test --package maverick-runtime-edge` | ✅ | ✅ green |
| 09-D-01 | D | 3 | CORE-03, RADIO-03 | — | Manual verification on physical hardware | manual | — | N/A | ⬜ manual |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- Existing infrastructure covers all phase requirements.

*No Wave 0 stubs needed — Rust test infrastructure already established.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| End-to-end auto-detection with physical SPI concentrator | CORE-03, RADIO-03 | Requires RAK Pi hardware with SX1302/SX1303 | Run `maverick-edge probe --summary` on target hardware, verify SPI detected and auto-mode selects SPI path |
| Feature flag gating for SPI without `--features spi` | CORE-03 | Build configuration test | Build without `--features spi`, verify Auto mode falls back to UDP |

*These items require physical hardware and cannot be tested in CI.*

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 45s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-04-17

---

## Validation Audit 2026-04-17

| Metric | Count |
|--------|-------|
| Gaps found | 0 |
| Resolved | 0 |
| Escalated | 0 |

---

## Requirements Coverage

| Requirement | Plan(s) | Status |
|-------------|---------|--------|
| CORE-03: Hardware auto-detection | A, B, C | ✅ COVERED |
| CORE-04: Platform capability reporting | A | ✅ COVERED |
| RADIO-01: SPI hardware detection | A | ✅ COVERED |
| RADIO-02: SPI device probing | A | ✅ COVERED |
| RADIO-03: Auto-enable SPI when detected | B, C | ✅ COVERED |
| RADIO-04: Operator guidance for SPI | C | ✅ COVERED |

---

## Test Summary

| Package | Tests | Status |
|---------|-------|--------|
| maverick-runtime-edge | 3 (SPI detection tests) | ✅ all pass |
| maverick-core | 48 | ✅ all pass |

*Phase 09 is NYQUIST-COMPLIANT — all automated requirements have verification coverage. Manual-only items (physical hardware testing) are documented above.*