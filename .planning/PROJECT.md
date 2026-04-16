# Maverick

## What This Is

Maverick is an offline-first, local LoRaWAN Network Server (LNS) designed for edge deployments where internet connectivity is unreliable or absent. It runs on any Linux hardware — from a Raspberry Pi to an x86 server — reads LoRa radio data directly or via a packet forwarder, and persists every uplink to a local SQLite database without depending on any cloud. The architecture is a small, rock-solid core surrounded by a fully extensible layer of isolated output plugins (HTTP, MQTT, cloud sync, web UI, etc.) that the community can build without ever touching the LNS core.

## Core Value

Never lose a LoRaWAN uplink — from radio to SQLite, data is preserved regardless of internet connectivity, extension state, or process restarts.

## Requirements

### Validated

- ✓ UDP GWMP ingest from Semtech packet forwarder — existing
- ✓ SQLite persistence for sessions, uplinks, and audit events — existing
- ✓ ABP session management via `lns-config.toml` config load — existing
- ✓ Terminal operator console (TUI) for interactive device and LNS management — existing
- ✓ Edge runtime CLI (`maverick-edge`) with hardware probe and install profiles — existing
- ✓ Multi-arch Linux builds (x86_64, aarch64, armv7) — existing
- ✓ Hexagonal architecture with port/adapter boundaries for all I/O — existing
- ✓ Circuit-breaker resilience for radio transport — existing

### Active

- [ ] MIC verification — validate LoRaWAN message integrity codes before accepting any frame
- [ ] FCnt 32-bit support — fix 16-bit FCnt truncation that breaks sessions after 65535 uplinks
- [ ] Process supervision and self-healing — LNS core auto-restarts on crash, never stays down
- [ ] Extension IPC boundary — local API surface so output plugins communicate with the core without coupling to it
- [ ] Direct SPI radio adapter — read from LoRa concentrator (SX1302/SX1303) without a packet forwarder
- [ ] Hardware compatibility registry — community-maintained list of tested hardware (verified, untested, unsupported)
- [ ] Unauthenticated UDP surface hardening — restrict or bind-protect the ingest port
- [ ] TUI device management — add/edit/remove devices and applications through the terminal UI, backed by SQLite

### Out of Scope

- OTAA join handling — Over-The-Air Activation, deferred to v2; ABP covers local deployments
- HTTP/MQTT output extensions — v2 feature; extension IPC boundary must exist first
- Cloud sync — v2; depends on extension IPC boundary and Maverick Cloud counterpart
- Web dashboard — v2 extension, separate process
- Payload decoders (Cayenne, custom JS/Lua) — v2 extension
- Multi-tenant / multi-user auth — v2; single-operator local deployment is the v1 target
- Windows / macOS runtime — Linux-only; other OSes via community effort if desired

## Context

Maverick targets operators who deploy LoRaWAN sensors in locations with poor or no internet connectivity: agricultural fields, industrial sites, remote infrastructure. The LNS must work entirely locally — no phone-home, no cloud dependency, no data loss when the WAN is down. When connectivity is available, a future extension will sync to Maverick Cloud (same team), but that is not the v1 constraint.

**Current codebase state (as of 2026-04-16):**
- The UDP ingest path works end-to-end: GWMP parse → session lookup → protocol validate → SQLite persist
- MIC verification is completely absent — any frame with a valid DevAddr and incrementing FCnt is accepted
- FCnt is parsed as 16-bit only; sessions permanently break after 65535 uplinks
- The UDP bind address defaults to `0.0.0.0:17000` with no authentication
- `DeviceRepository` and `DownlinkRepository` port traits are defined but have no adapter implementation
- OTAA join is absent; only pre-configured sessions via `config load` are operational
- Cloud sync contracts exist in `maverick-extension-contracts` but are not wired to anything
- `maverick-cloud-core` and `maverick-extension-contracts` define the wire schema for future cloud sync
- The TUI (`maverick-extension-tui`) is a separate binary that shells out to `maverick-edge` — good isolation model

**Hardware target for v1:** Raspberry Pi (armv7/aarch64) with RAK LoRa concentrator HAT (SX1302/SX1303 chipset).

**Extension isolation principle:** Output plugins (HTTP, MQTT, cloud sync, web UI) must run as separate processes. The LNS core (`maverick-edge`) must never be blocked, panicked, or slowed by extension failures. Extensions communicate with the core via a local IPC surface (e.g., a local HTTP API or Unix socket). This is the same model as the existing TUI.

## Constraints

- **Tech Stack**: Rust — no runtime changes; hexagonal architecture must be maintained
- **Offline-first**: Zero cloud calls in the core runtime; all persistence is local SQLite
- **Process isolation**: Extensions are separate processes, never in-process plugins in the core
- **Hardware**: Linux only; must run on armv7 (Raspberry Pi 3) with ≤512 MB RAM
- **Resilience**: The LNS core must be supervised and self-healing; packet loss = failure
- **Compatibility**: Existing `lns-config.toml` format must remain valid; no breaking config changes in v1

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Hexagonal architecture | Enables swapping radio adapters (UDP → SPI) without touching use cases | ✓ Good |
| SQLite with bundled feature | No system lib dependency; works on any Linux target | ✓ Good |
| Extensions as separate processes | LNS core stability isolated from plugin failures | — Pending |
| Extension IPC surface (local API) | Enables community plugins without in-process coupling | — Pending |
| MIC verification in v1 | Without it, Maverick is not a real LNS; any node can inject fake data | — Pending |
| FCnt 32-bit fix in v1 | 16-bit limit makes Maverick unusable for devices that stay deployed > 65535 uplinks | — Pending |

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
*Last updated: 2026-04-16 after initialization*
