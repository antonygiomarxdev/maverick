# Roadmap: Maverick LNS

## Milestones

- ✅ **v1.0 MVP** — Phases 1-10 (shipped 2026-04-17)
- 🚧 **v1.1** — Next release (TBD)

## Phases

<details>
<summary>✅ v1.0 MVP (Phases 1-10) — SHIPPED 2026-04-17</summary>

- [x] Phase 1: Protocol Correctness (6/6 plans) — completed 2026-04-16
- [x] Phase 2: Radio Abstraction & SPI (4/4 plans) — completed 2026-04-16
- [x] Phase 3: Protocol Security (2/2 plans) — completed 2026-04-16
- [x] Phase 3.1: Class A Downlink (INSERTED) — completed 2026-04-17
- [x] Phase 4: Process Supervision — completed 2026-04-17
- [x] Phase 5: TUI Device Management — completed 2026-04-16
- [x] Phase 6: Phase 4 Verification & Artifacts (1/1 plans) — completed 2026-04-17
- [x] Phase 7: Phase 5 Verification & Artifacts (1/1 plans) — completed 2026-04-17
- [x] Phase 8: Hardware Testing (RAK Pi) (10/10 plans) — completed 2026-04-17
- [x] Phase 9: Hardware Auto-Detection & SPI Enable (4/4 plans) — completed 2026-04-17
- [x] Phase 10: libloragw SPI Integration (2/2 plans) — completed 2026-04-17

</details>

### 🚧 v1.1 (Next Release)

- [x] Phase 11: TBD (completed 2026-04-17)

## Progress

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. Protocol Correctness | v1.0 | 6/6 | Complete | 2026-04-16 |
| 2. Radio Abstraction & SPI | v1.0 | 4/4 | Complete | 2026-04-16 |
| 3. Protocol Security | v1.0 | 2/2 | Complete | 2026-04-16 |
| 3.1 Class A Downlink | v1.0 | 1/1 | Complete | 2026-04-17 |
| 4. Process Supervision | v1.0 | 1/1 | Complete | 2026-04-17 |
| 5. TUI Device Management | v1.0 | 1/1 | Complete | 2026-04-16 |
| 6. Phase 4 Verification | v1.0 | 1/1 | Complete | 2026-04-17 |
| 7. Phase 5 Verification | v1.0 | 1/1 | Complete | 2026-04-17 |
| 8. Hardware Testing (RAK Pi) | v1.0 | 10/10 | Complete | 2026-04-17 |
| 9. Hardware Auto-Detection | v1.0 | 4/4 | Complete | 2026-04-17 |
| 10. libloragw SPI Integration | v1.0 | 2/2 | Complete | 2026-04-17 |
| 11. TBD | v1.1 | 3/3 | Complete   | 2026-04-17 |

## Known Tech Debt (v1.0)

| Item | Phase | Status | Notes |
|------|-------|--------|-------|
| SEC-02 (SQLite key encryption) | Phase 4 | Deferred to v1.1 | Domain model refactor [u8;16]→Vec<u8> required |
| DEV-01 (TUI wizard tests) | Phase 5 | Partial | Wizard exists, TOML workaround available |
| DEV-03 (device remove CLI) | Phase 5 | Partial | TOML edit + reload workaround sufficient |
| CORE-03 (hardware probe verification) | Phase 5 | Partial | Probe runs and visible, not formally verified |
| DWNL-01..DWNL-06 integration | Phase 3.1 | Deferred | DownlinkScheduler needs wiring; SPI TX deferred |

## Backlog

See `.planning/milestones/v1.0-ROADMAP.md` for full v1.0 phase details.

### Phase 11: Auto-update mechanism for ARM gateways

**Goal:** [To be planned]
**Requirements**: TBD
**Depends on:** Phase 10
**Plans:** 3/3 plans complete

Plans:
- [x] TBD (run /gsd-plan-phase 11 to break down) (completed 2026-04-17)

### Phase 12: Release CI hardening and update URL configuration

**Goal:** [To be planned]
**Requirements**: TBD
**Depends on:** Phase 11
**Plans:** 1/1 plans complete

Plans:
- [x] TBD (run /gsd-plan-phase 12 to break down) (completed 2026-04-17)

### Phase 13: CI SPI support with libloragw cross-compilation

**Goal:** [To be planned]
**Requirements**: TBD
**Depends on:** Phase 12
**Plans:** 0 plans

Plans:
- [ ] TBD (run /gsd-plan-phase 13 to break down)

---

*Roadmap reorganized: 2026-04-17 after v1.0 milestone completion*
