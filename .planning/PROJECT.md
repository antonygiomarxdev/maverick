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

- ✓ Direct SPI radio adapter (SX1302/SX1303) — gateway + LNS in one
- ✓ UDP/GWMP ingest from packet forwarders (backward compatible)
- ✓ SQLite persistence for sessions, uplinks, and audit events
- ✓ ABP session management via `lns-config.toml` config load
- ✓ Terminal operator console (TUI) as optional extension
- ✓ Edge runtime CLI (`maverick-edge`) with hardware probe and install profiles
- ✓ Multi-arch Linux builds (x86_64, aarch64, armv7)
- ✓ Hexagonal architecture with port/adapter boundaries for all I/O
- ✓ Circuit-breaker resilience for radio transport
- ✓ Extensions as separate processes (isolated from core)

### Active

- [ ] MIC verification — validate LoRaWAN message integrity codes before accepting any frame
- [ ] FCnt 32-bit support — fix 16-bit FCnt truncation that breaks sessions after 65535 uplinks
- [ ] Extension IPC boundary — local API surface so extensions communicate with the core
- [ ] Hardware compatibility registry — community-maintained list of tested hardware
- [ ] Unauthenticated UDP surface hardening — restrict or bind-protect the ingest port
- [ ] TUI device management — add/edit/remove devices through the terminal UI

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
│  │  Radio  │  │ SQLite  │  │   CLI   │  │  IPC    │  │
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

Maverick targets operators who deploy LoRaWAN sensors in locations with poor or no internet connectivity: agricultural fields, industrial sites, remote infrastructure. The stack must work entirely locally — no phone-home, no cloud dependency, no data loss when WAN is down.

**Current codebase state (as of 2026-04-16):**
- Direct SPI radio adapter exists as `maverick-adapter-radio-spi` crate
- UDP/GWMP ingest path works end-to-end: GWMP parse → session lookup → protocol validate → SQLite persist
- MIC verification is completely absent — any frame with a valid DevAddr and incrementing FCnt is accepted
- FCnt is parsed as 16-bit only; sessions permanently break after 65535 uplinks
- `DeviceRepository` and `DownlinkRepository` port traits are defined but have no adapter implementation
- Cloud sync contracts exist but are not wired to anything
- The TUI is a separate binary that shells out to `maverick-edge` — good isolation model

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
| MIC verification in v1 | Without it, Maverick is not a real LNS | 🔲 Pending |
| FCnt 32-bit fix in v1 | 16-bit limit breaks devices after 65535 uplinks | 🔲 Pending |

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
*Last updated: 2026-04-16*
