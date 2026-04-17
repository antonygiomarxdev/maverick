# Maverick Roadmap

Date: 2026-04-16
Status: Active — **public beta** (`0.x` SemVer tags; see [`docs/release-policy.md`](docs/release-policy.md))

## North Star

**LoRaWAN. Offline. Always.**

A self-contained LoRaWAN gateway + network server that runs where nothing else does. Your data never dies.

## Principles

1. **Reliability above all** — The LNS core never falls, never loses data, never blocks.
2. **Zero cloud dependency** — All persistence is local. Zero calls to external services.
3. **Everything is optional** — TUI, dashboard, HTTP, MQTT, AI — operator chooses.
4. **Extension isolation** — Extensions are separate processes. Core never affected.
5. **Community-driven** — Opensource. Contributions welcome everywhere.
6. **AI-compatible** — Door open for AI extensions when hardware allows.

## v1 Milestone: Functional LNS-Gateway

**Goal:** Ship a complete, reliable, self-contained LoRaWAN stack.

---

## Phase 01: Protocol Correctness
**Status:** ✅ Complete (partial)

LoRaWAN 1.0.x protocol implementation.

**Incomplete:**
- MIC verification — pending
- FCnt 32-bit support — pending

These are security-critical and will be addressed in Phase 03.

## Phase 02: Radio Abstraction & SPI
**Status:** ✅ Complete

Direct SPI radio adapter for SX1302/SX1303.
Gateway + LNS in one device.

## Phase 03: Protocol Security
**Status:** 🔲 Next

MIC verification and FCnt 32-bit support.

**Why:** Without MIC, any node can inject fake data. Without FCnt 32-bit, sessions break after 65535 uplinks.

**Goals:**
- MIC verification — validate every uplink before accepting
- FCnt 32-bit — support full 32-bit frame counter
- FRMPayload decryption — AES-128 decript in end-device mode

**Exit criteria:**
- [ ] MIC verification passes for valid frames
- [ ] MIC verification rejects forged frames
- [ ] FCnt 32-bit works across rollover boundary
- [ ] Protocol state machine handles all edge cases

## Phase 04: Class A Downlink
**Status:** 🔲 Queued

Bidirectional LoRaWAN communication.

**Goals:**
- Downlink TX window timing (RX1, RX2)
- Downlink scheduling and queue management
- DeviceRepository port implementation
- DownlinkRepository port implementation

**Exit criteria:**
- [ ] Can send downlink to device after uplink
- [ ] RX1/RX2 timing correct per region
- [ ] Downlink queue persists across restarts

## Phase 05: Extension IPC
**Status:** 🔲 Queued

Local API surface for extensions.

**Why:** Community needs a defined interface to build extensions without coupling to core.

**Goals:**
- Define extension IPC protocol
- Implement core-side listener (HTTP, Unix socket, or similar)
- Document extension contract
- Create example extension template

**Exit criteria:**
- [ ] Extensions can communicate with core via IPC
- [ ] Extension contracts documented
- [ ] Example/template extension available

## Phase 06: Process Supervision
**Status:** 🔲 Queued

Auto-restart, self-healing, reliability.

**Goals:**
- Core process supervised (auto-restart on crash)
- Extension process monitoring
- Health checks and status reporting
- Graceful shutdown handling

**Exit criteria:**
- [ ] Core restarts automatically on crash
- [ ] Extension failures don't affect core
- [ ] `maverick-edge health` reports accurate status

## Phase 07: Community-Ready
**Status:** 🔲 Queued

Prepare for v1.0 release.

**Goals:**
- Hardware compatibility registry
- Extension templates and documentation
- Contributor guide
- Release artifacts for all targets

**Exit criteria:**
- [ ] Hardware registry documents tested configurations
- [ ] Extension SDK/template available
- [ ] CONTRIBUTING.md complete
- [ ] v0.1.0 or v1.0.0 release tagged

---

## Post-v1 (Backlog)

### Maverick Cloud Sync
- Edge pushes to cloud when connectivity available
- MQTT or HTTPS with queue
- Eventual consistency model

### Extensions (Official)
- `maverick-tui` — terminal console
- `maverick-dashboard` — web UI
- `maverick-http` — HTTP webhooks
- `maverick-mqtt` — MQTT integration
- `maverick-ai` — AI analytics (API-based)

### OTAA Join
- Over-The-Air activation support
- Deferred to v2

### Multi-Region
- Full region support beyond EU868

---

## Quality Gates

Before closing each phase, verify:
- [ ] Code follows Rust clean code standards
- [ ] Hexagonal architecture maintained
- [ ] `cargo fmt` + `cargo clippy` pass
- [ ] No cloud dependencies in core
- [ ] Extensions remain isolated

See: `.planning/QUALITY-CHECKLIST.md`

---

## Authoritative References

- [`VISION.md`](VISION.md) — Project vision and principles
- [`.planning/QUALITY-CHECKLIST.md`](.planning/QUALITY-CHECKLIST.md) — Quality standards
- [`docs/00-product-intent.md`](docs/00-product-intent.md)
- [`docs/01-execution-plan.md`](docs/01-execution-plan.md)
- [`docs/03-operating-model.md`](docs/03-operating-model.md)
