# Roadmap: Maverick LNS

## Milestones

- ✅ **v1.0 MVP** — Phases 1-13 (shipped 2026-04-17)
- 🚧 **v1.1** — Next release (TBD)

## Phases

<details>
<summary>✅ v1.0 MVP (Phases 1-13) — SHIPPED 2026-04-17</summary>

- [x] Phase 1: Protocol Correctness (6/6 plans) — completed 2026-04-16
- [x] Phase 2: Radio Abstraction & SPI (4/4 plans) — completed 2026-04-16
- [x] Phase 3: Protocol Security (2/2 plans) — completed 2026-04-16
- [x] Phase 3.1: Class A Downlink (INSERTED) — completed 2026-04-17
- [x] Phase 4: Process Supervision (1/1 plan) — completed 2026-04-17
- [x] Phase 5: TUI Device Management (1/1 plan) — completed 2026-04-16
- [x] Phase 6: Phase 4 Verification & Artifacts (1/1 plan) — completed 2026-04-17
- [x] Phase 7: Phase 5 Verification & Artifacts (1/1 plan) — completed 2026-04-17
- [x] Phase 8: Hardware Testing (RAK Pi) (10/10 plans) — completed 2026-04-17
- [x] Phase 9: Hardware Auto-Detection & SPI Enable (4/4 plans) — completed 2026-04-17
- [x] Phase 10: libloragw SPI Integration (2/2 plans) — completed 2026-04-17
- [x] Phase 11: Auto-Update Mechanism for ARM Gateways (3/3 plans) — completed 2026-04-17
- [x] Phase 12: Release CI Hardening and Update URL Configuration (1/1 plan) — completed 2026-04-17
- [x] Phase 13: CI SPI Support with libloragw Cross-Compilation (1/1 plan) — completed 2026-04-17

</details>

### 🚧 v1.1 (Next Release)

- [ ] Phase 14: Fix Cross-Compiled SPI Binary Initialization (0/1 plans) — in progress
- [ ] Phase 15: Auto-Update Binary Verification (0/1 plans) — planned
- [ ] Phase 16: Structured File Logging & Observability (0/1 plans) — planned
- [ ] Phase 17: TBD (remaining v1.1 features)

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
| 11. Auto-Update Mechanism | v1.0 | 3/3 | Complete | 2026-04-17 |
| 12. Release CI Hardening | v1.0 | 1/1 | Complete | 2026-04-17 |
| 13. CI SPI Cross-Compilation | v1.0 | 1/1 | Complete | 2026-04-17 |
| 14. Fix Cross-Compiled SPI Binary | v1.1 | 0/1 | In Progress | — |

## Known Tech Debt (v1.0)

| Item | Phase | Status | Notes |
|------|-------|--------|-------|
| SEC-02 (SQLite key encryption) | Phase 4 | Deferred to v1.1 | Domain model refactor [u8;16]→Vec<u8> required |
| DEV-01 (TUI wizard tests) | Phase 5 | Partial | Wizard exists, TOML workaround available |
| DEV-03 (device remove CLI) | Phase 5 | Partial | TOML edit + reload workaround sufficient |
| CORE-03 (hardware probe verification) | Phase 5 | Partial | Probe runs and visible, not formally verified |
| DWNL-01..DWNL-06 integration | Phase 3.1 | Deferred | DownlinkScheduler needs wiring; SPI TX deferred |
| RADIO-01 (full SPI RX/TX on real ARM) | Phase 10 | Pending | Field testing on RAK Pi required |
| 09-D (auto-detection verification) | Phase 9 | Pending | Integration tests need hardware or mocks |

## Backlog

### v1.1 Candidates

- SEC-02: SQLite key encryption (SQLCipher)
- DWNL-01..DWNL-06: Full downlink SPI TX wiring
- OTAA join handling (deferred from v1)
- HTTP/MQTT/webhook extension IPC boundary
- Payload decoders (Cayenne, custom JS/Lua)
- Multi-tenant auth

---

*Roadmap reorganized: 2026-04-22 after v1.0 milestone completion*
