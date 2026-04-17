# Maverick

**LoRaWAN. Offline. Always.**

## What This Is

Maverick is an **offline-first, self-contained LoRaWAN stack** — a gateway and LNS in one. It runs on edge hardware (Raspberry Pi and similar), reads directly from LoRa radios (SX1302/SX1303) via SPI, persists every uplink to local SQLite, and keeps running regardless of internet connectivity, extension failures, or process restarts.

The architecture is a rock-solid core surrounded by fully isolated, optional extensions (TUI, dashboard, HTTP, MQTT, webhooks, AI) that the community can build without ever touching the LNS core.

## Core Value

**Your LoRaWAN data never dies** — from radio to SQLite, data is preserved regardless of internet connectivity, extension state, or process restarts.

## Principles

1. **Reliability above all** — The LNS core never falls, never loses data, never blocks due to external causes. If the dashboard fails, the LNS continues. If internet goes down, the LNS continues. If an extension has a bug, the LNS continues.

2. **Zero cloud dependency** — The runtime has zero calls to external services. All persistence is local.

3. **Everything is optional** — TUI, dashboard, HTTP, MQTT, AI — the operator chooses what to install. Nothing is forced.

4. **Extension isolation** — Extensions are separate processes. They can fail, crash, or misbehave without affecting the core.

5. **Community-driven** — Opensource. Contributions welcome in core, extensions, hardware compatibility, documentation, and AI integrations.

6. **AI-compatible** — Extensions can leverage AI APIs (Claude, OpenAI, etc.). The door is open for local ML when hardware allows.

## Requirements

### Validated

- ✓ Direct SPI radio adapter (SX1302/SX1303) — gateway + LNS in one (Phase 2, 10)
- ✓ UDP/GWMP ingest from packet forwarders (backward compatible) (Phase 2)
- ✓ SQLite persistence for sessions, uplinks, and audit events (Phase 1)
- ✓ ABP session management via `lns-config.toml` config load (Phase 1)
- ✓ Terminal operator console (TUI) as optional extension (Phase 5)
- ✓ Edge runtime CLI (`maverick-edge`) with hardware probe and install profiles (Phase 5)
- ✓ Multi-arch Linux builds (x86_64, aarch64, armv7) (Phase 1)
- ✓ Hexagonal architecture with port/adapter boundaries for all I/O (Phase 1, 2)
- ✓ Circuit-breaker resilience for radio transport (Phase 2)
- ✓ Extensions as separate processes (isolated from core) (Phase 5)
- ✓ MIC verification (AES-128 CMAC) on every uplink frame (Phase 1)
- ✓ FCnt 32-bit reconstruction from 16-bit wire value (Phase 1)
- ✓ NwkSKey and AppSKey stored per session in SQLite (Phase 1)
- ✓ AppSKey payload decryption (AES-128 CTR) persisted to SQLite (Phase 1)
- ✓ Region inference for AU915 and AS923 (Phase 1)
- ✓ Duplicate uplink detection and discard (Phase 1)
- ✓ UDP bind address configurable, defaults to 127.0.0.1:17000 (Phase 1)
- ✓ SQLite Mutex poison-free error handling (Phase 1)
- ✓ Clean shutdown with WAL checkpoint (Phase 1)
- ✓ UplinkSource port trait for radio backend abstraction (Phase 2)
- ✓ Hardware compatibility registry (TOML) (Phase 2)
- ✓ Radio backend selectable via config (SPI or UDP) (Phase 2)
- ✓ Class A downlink scheduling (RX1/RX2) with LinkCheckAns (Phase 3.1)
- ✓ Downlink queue persists to SQLite (survives restart) (Phase 3.1)
- ✓ systemd Restart=always supervision (Phase 4)
- ✓ Systemd watchdog for hung process detection (Phase 4)
- ✓ Hardware probe on startup (CPU, RAM, storage, arch) (Phase 5)
- ✓ Device list with last-seen and uplink count (Phase 5)
- ✓ lns-config.toml import for bulk provisioning (Phase 5)
- ✓ Autoprovision-pending device promotion via TUI (Phase 5)

### Active

- [ ] SEC-02: NwkSKey/AppSKey SQLite encryption — domain model refactor deferred to v1.1
- [ ] DEV-01: TUI device wizard automated tests
- [ ] DEV-03: Direct `device remove` CLI command
- [ ] DWNL-01..DWNL-06: Full downlink integration (SPI TX, runtime wiring)
- [ ] RADIO-01: Full SPI RX/TX on real ARM hardware

### Out of Scope (v1)

- OTAA join handling — deferred to v2; ABP covers local deployments
- HTTP/MQTT/webhook extensions — v2 feature; extension IPC boundary must exist first
- Maverick Cloud sync — v2; depends on extension IPC and cloud counterpart
- Web dashboard — v2 extension
- Payload decoders (Cayenne, custom JS/Lua) — v2 extension
- Multi-tenant / multi-user auth — v2; single-operator local deployment is v1 target
- Windows / macOS runtime — Linux-only

## Hardware

- **Target**: Raspberry Pi 3/4 (armv7/aarch64) with RAK LoRa concentrator HAT (SX1302/SX1303 chipset)
- **Minimum**: armv7, 512 MB RAM, Linux
- **Multi-arch**: x86_64, aarch64, armv7 binaries
- **Storage**: SQLite on local storage; configurable circular buffer for SD card protection

## Sync with Maverick Cloud

**Model:** Many edges → one cloud (architecture open to many→many).

- The edge **pushes** when connectivity is available
- Connection can be intermittent, slow, or absent for days
- Efficient protocol (MQTT or HTTPS with queue)
- Edge maintains queue of pending events with timestamps
- **Eventual consistency**: when there's network, it syncs; when there isn't, it accumulates
- Auth: token per edge + TLS
- Conflicts: **edge wins** — edge is source of truth for its local data

## Extensions Model

```
┌─────────────────────────────────────────────────────────┐
│                     maverick-edge                       │
│                (Gateway + LNS — always up)               │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐  │
│  │  Radio  │  │ SQLite  │  │   CLI   │  │   IPC   │  │
│  │   SPI   │  │persist  │  │   mgmt  │  │ surface │  │
│  └─────────┘  └─────────┘  └─────────┘  └─────────┘  │
└─────────────────────────────────────────────────────────┘
                               │
                               │ (Unix pipes / TCP / HTTP local)
                               ▼
         ┌──────────────────┼───────────────────┬──────────┐
         ▼                  ▼                   ▼          ▼
   ┌──────────┐      ┌──────────┐      ┌──────────┐ ┌────────┐
   │   TUI    │      │   HTTP    │      │   MQTT   │ │   AI   │
   │(optional)│      │  outbound │      │  outbound│ │(optional)│
   └──────────┘      └──────────┘      └──────────┘ └────────┘
```

**All extensions are optional and separate processes.** The operator chooses what to install and configure.

## Context

**v1.0 shipped** — 2026-04-17

**Current codebase state (as of 2026-04-17):**
- Direct SPI radio adapter (`maverick-adapter-radio-spi`) with libloragw FFI bindings
- UDP/GWMP ingest path fully functional: GWMP parse → session lookup → MIC verify → protocol validate → SQLite persist
- MIC verification fully implemented with LoRaWAN spec test vectors
- FCnt 32-bit support implemented — sessions survive beyond 65535 uplinks
- NwkSKey and AppSKey stored per session, used for MIC computation and payload decryption
- `DeviceRepository` and `DownlinkRepository` port traits with SQLite adapters
- Class A downlink scheduler designed (RX1/RX2 timing) but not yet wired to runtime
- TUI device management complete with wizard-based add/edit/remove
- Systemd supervision with watchdog support
- Hardware probe runs on startup and surfaces in TUI

**v1.0 Stats:**
- 217 files changed, 28,488 insertions, 778 deletions
- 11 phases, 33 plans completed
- 28,533 lines of Rust/TOML code

## Constraints

- **Tech Stack**: Rust — hexagonal architecture must be maintained
- **Offline-first**: Zero cloud calls in the core runtime; all persistence is local SQLite
- **Process isolation**: Extensions are separate processes, never in-process plugins
- **Hardware**: Linux only; must run on armv7 (Raspberry Pi 3) with ≤512 MB RAM
- **Resilience**: Packet loss = failure; the core must be supervised and self-healing
- **Compatibility**: Existing `lns-config.toml` format must remain valid; no breaking config changes in v1

## Key Decisions

| Decision | Rationale | Status |
|----------|-----------|--------|
| Gateway + LNS in one | Simplifies deployment; one device does everything | ✅ Good |
| Direct SPI radio | No separate gateway hardware needed | ✅ Good |
| Hexagonal architecture | Enables swapping radio adapters (SPI ↔ UDP) without touching use cases | ✅ Good |
| SQLite with bundled rusqlite | No system lib dependency; works on any Linux target | ✅ Good |
| Extensions as separate processes | Core stability isolated from plugin failures | ✅ Good |
| Extensions are optional | No bloat; operator chooses what to install | ✅ Good |
| Edge as source of truth | No conflicts; cloud receives, edge controls | ✅ Good |
| Eventual sync | Works with intermittent connectivity; realtime is nice-to-have | ✅ Good |
| MIC verification in v1 | Without it, Maverick is not a real LNS | ✅ Validated |
| FCnt 32-bit fix in v1 | 16-bit limit breaks devices after 65535 uplinks | ✅ Validated |
| Decimal phase numbering | Clear semantics for inserted phases | ✅ Good |
| Class A downlink deferred | SPI TX not ready; design exists but needs wiring | ⚠️ Revisit in v1.1 |
| SEC-02 deferred to v1.1 | Domain model refactor required before SQLCipher | 🔲 Pending |

---

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd-transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd-complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-04-17 after v1.0 milestone*
