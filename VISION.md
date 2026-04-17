# Maverick — Vision

**LoRaWAN. Offline. Always.**

Maverick is an offline-first, self-contained LoRaWAN stack that treats data integrity as non-negotiable: built to run where nothing else does, extensible by a community that makes it grow.

---

## What It Is

Maverick is a LoRaWAN Network Server (LNS) designed for edge deployments where connectivity is null, intermittent, or unstable.

**Core value:** Never lose an uplink — from radio to SQLite, data is preserved regardless of internet connectivity, extension failures, or process restarts.

---

## What It Is and What It Is Not

### It Is

- A **complete stack** that installs and works: LNS + direct radio (SX1302/3), no external dependencies
- **Offline-first**: zero calls to the cloud in the core runtime; all persistence is local
- **Extensible**: everything is optional — TUI, dashboard, HTTP, MQTT, webhooks, AI — installed and configured by the operator
- **Isolated**: extensions are separate processes that never affect the LNS core stability
- **Opensource**: the community contributes to core, extensions, documentation, hardware compatibility
- **AI-compatible**: official extensions that leverage AI APIs (Claude, OpenAI); door open for local ML on more capable hardware

### It Is Not

- A cloud service
- Connectivity-dependent
- A closed or monolithic product
- Designed for Windows or macOS (Linux only)
- A replacement for TTN/The Things Stack (can integrate with them)

---

## Principles

### 1. Reliability Above All

The LNS core never falls, never loses data, never blocks due to external causes. If the dashboard fails, the LNS continues. If internet goes down, the LNS continues. If an extension has a bug, the LNS continues.

### 2. Zero Cloud Dependency

The runtime has zero calls to external services. All persistence is local.

### 3. Everything Is Optional

TUI, dashboard, HTTP, MQTT, AI — the operator chooses what to install. Nothing is forced.

### 4. Extension Isolation

Extensions are separate processes. They can fail, crash, or misbehave without affecting the core.

### 5. Community-Driven

Opensource. Contributions welcome in core, extensions, hardware compatibility, documentation, and AI integrations.

### 6. AI-Compatible

Extensions can leverage AI APIs (Claude, OpenAI, etc.). Door open for local ML when hardware allows.

---

## Extension Architecture

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
  │(optional)│      │  outbound │      │ outbound │ │(optional)│
  └──────────┘      └──────────┘      └──────────┘ └────────┘
```

**All extensions are optional and separate processes.** The operator chooses what to install.

---

## Sync with Maverick Cloud

**Model:** Many edges → one cloud (open to many→many).

- The edge **pushes** when connectivity is available
- Connection can be intermittent, slow, or absent for days
- Efficient protocol (MQTT or HTTPS with queue)
- Edge maintains queue of pending events with timestamps
- **Eventual consistency**: when there's network, it syncs; when there isn't, it accumulates
- Auth: token per edge + TLS
- Conflicts: **edge wins** — edge is source of truth for its local data

### Sync Data

Configurable, but by default:
- Uplinks + sessions (core)
- Operational metrics
- Logs (optional, not by default)

---

## Installation and Setup

### First Time (Interactive CLI)

```bash
maverick install
# LoRaWAN region
# Detected radio hardware
# Extensions to install (none by default)
# Initial config / credentials
```

Supports headless deployment (SSH + config file or serial for initial setup).

### Ongoing Operation

- CLI: `maverick device add ...`, `maverick config set ...`
- Extensions configured via their own CLI or config file
- All manageable via SSH

### Updates

TBD (OTA or manual).

---

## Data Retention

- **Core value**: no uplink is ever lost
- By default: persists indefinitely in local SQLite
- Configurable circular buffer to protect limited storage (SD cards)
- Cleanup strategy: configurable, non-destructive by default

---

## Hardware

- **Target**: Raspberry Pi 3/4 (armv7/aarch64) with RAK LoRa concentrator HAT (SX1302/3)
- **Minimum**: armv7, 512 MB RAM, Linux
- **Multi-arch**: x86_64, aarch64, armv7 binaries
- **Extensible**: community validates and extends hardware compatibility

---

## Community and Opensource

Maverick is opensource and contribution is welcome in all areas:

- **Core**: bug fixes, features, protocol compliance
- **Extensions**: official and community-driven
- **Hardware**: drivers, compatibility testing
- **Documentation**: guides, tutorials, case studies
- **AI integrations**: new providers, local ML

---

## Roadmap (Current Status)

| Phase | Description | Status |
|-------|-------------|--------|
| 01 | Protocol Correctness | ✅ Complete (partial) |
| 02 | Radio Abstraction & SPI | ✅ Complete |
| 03 | Protocol Security | 🔲 Pending |
| 04 | Class A Downlink | 🔲 Pending |
| 05 | Extension IPC | 🔲 Pending |
| 06 | Process Supervision | 🔲 Pending |
| 07 | Observability | 🔲 Pending |
| 08 | Community-Ready | 🔲 Pending |

---

## Technical Constraints

- **Tech Stack**: Rust — hexagonal architecture, clean code
- **Offline-first**: zero cloud calls in core
- **Process isolation**: extensions are separate processes
- **Hardware**: Linux only; must run on armv7 (Raspberry Pi 3) with ≤512 MB RAM
- **Compatibility**: `lns-config.toml` format remains valid; no breaking changes in v1

---

_Last updated: 2026-04-16_
