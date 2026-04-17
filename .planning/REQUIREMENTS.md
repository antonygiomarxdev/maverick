# Requirements: Maverick LNS

**Defined:** 2026-04-16
**Core Value:** Never lose a LoRaWAN uplink — from radio to SQLite, data is preserved regardless of internet connectivity, extension state, or process restarts.

---

## v1 Requirements

### Offline-First Core

- [ ] **CORE-01**: `maverick-edge` makes zero external HTTP/DNS calls — all core LNS functions work with no internet connectivity
- [ ] **CORE-02**: Every accepted uplink is written to SQLite before any acknowledgment or downstream processing — no uplink lost to in-memory state
- [ ] **CORE-03**: Hardware probe detects available memory, CPU arch, and storage automatically — no manual architecture config required from operator
- [ ] **CORE-04**: Hardware compatibility registry (TOML) ships with verified/untested/unsupported classification — community can contribute new entries without code changes

### Protocol Correctness

- [ ] **PROT-01**: LNS verifies MIC (AES-128 CMAC) on every uplink frame before accepting it
- [ ] **PROT-02**: LNS reconstructs 32-bit FCnt from 16-bit wire value using session counter high bits
- [ ] **PROT-03**: LNS stores NwkSKey and AppSKey per session and uses them for MIC verification and payload decryption
- [ ] **PROT-04**: LNS decrypts uplink payload with AppSKey (AES-128 CTR) and persists decrypted payload
- [ ] **PROT-05**: Region inference correctly identifies AU915 and AS923 without shadowing by US915
- [ ] **PROT-06**: Duplicate uplink frames (same DevAddr + FCnt within a deduplication window) are detected and discarded — only the first copy is persisted

### LoRaWAN Class A (Downlink)

- [x] **DWNL-01**: LNS schedules Class A downlink in RX1 window (1s after uplink end) when downlink is queued
- [x] **DWNL-02**: LNS falls back to RX2 window (2s after uplink end) if RX1 transmission fails
- [x] **DWNL-03**: LNS sends ACK flag in downlink for confirmed uplinks
- [x] **DWNL-04**: Downlink queue persists to SQLite (survives process restart)
- [x] **DWNL-05**: Downlink transmission uses precise hardware timestamp from concentrator to hit RX1/RX2 windows within LoRaWAN Class A timing tolerance
- [x] **DWNL-06**: LNS parses LinkCheckReq MAC command from FOpts and responds with LinkCheckAns in next downlink

### Radio Hardware (SPI Direct)

- [ ] **RADIO-01**: Maverick reads LoRa frames directly from SX1302/SX1303 concentrator via SPI on Raspberry Pi without requiring an external packet forwarder
- [ ] **RADIO-02**: SPI radio adapter implements the `UplinkSource` port trait alongside the existing UDP adapter
- [ ] **RADIO-03**: Radio backend is selectable via config (SPI or UDP) — UDP remains for dev/testing/simulator use
- [ ] **RADIO-04**: Hardware compatibility registry documents RAK Pi as verified-supported hardware

### Reliability & Stability

- [ ] **RELI-01**: SQLite Mutex cannot be permanently poisoned by internal errors — all `.expect()` calls inside lock scope replaced with `?`-propagation
- [ ] **RELI-02**: Process shutdown always checkpoints SQLite WAL before exit — `process::exit` replaced with clean shutdown path
- [ ] **RELI-03**: `maverick-edge` process is supervised by systemd with `Restart=always` — restarts automatically after crash
- [ ] **RELI-04**: Systemd watchdog (`WatchdogSec`) detects hung processes (not just crashes) and triggers restart
- [ ] **RELI-05**: `UplinkSource` port trait abstracts radio backend so ingest loop is radio-agnostic

### Security

- [ ] **SEC-01**: UDP ingest bind address is configurable — default changed from `0.0.0.0:17000` to `127.0.0.1:17000`
- [ ] **SEC-02**: NwkSKey and AppSKey stored in SQLite with SQLite-level encryption or access controls (not plaintext in schema)

### Device Management

- [ ] **DEV-01**: Operator can add a new device (DevEUI, DevAddr, NwkSKey, AppSKey, region, application) via TUI without editing TOML manually
- [ ] **DEV-02**: Operator can list all registered devices with status (last seen, uplink count) via TUI
- [ ] **DEV-03**: Operator can remove a device via TUI
- [ ] **DEV-04**: `lns-config.toml` import remains supported for bulk device provisioning
- [ ] **DEV-05**: Operator can view autoprovision-pending devices (unknown DevAddr frames) in TUI and promote them to registered devices

---

## v2 Requirements

### Extension System

- **EXT-01**: `maverick-edge` exposes local HTTP API (`127.0.0.1:17001`) for extension plugins to query state
- **EXT-02**: Extensions can subscribe to uplink events via SSE stream without blocking the ingest loop
- **EXT-03**: Each extension maintains an independent cursor in `sync_cursors` table for catch-up on reconnect

### Output Plugins

- **OUT-01**: HTTP webhook output plugin forwards decoded uplinks to configurable endpoint
- **OUT-02**: MQTT output plugin publishes uplinks to local or remote broker
- **OUT-03**: Cloud sync plugin replicates uplinks to Maverick Cloud using `SyncBatchEnvelopeV1` contracts

### OTAA

- **OTAA-01**: LNS handles JoinRequest and sends JoinAccept for OTAA device activation
- **OTAA-02**: Session keys derived from AppKey, DevNonce, and join nonces per LoRaWAN 1.0.x spec

### Advanced Features

- **ADV-01**: Web dashboard (separate process) for device and uplink visibility
- **ADV-02**: Payload decoder plugin system (Cayenne LPP, custom)
- **ADV-03**: MAC command handling (LinkCheckReq/Ans, DevStatusReq)

---

## Out of Scope

| Feature | Reason |
|---------|--------|
| Multi-tenant / multi-user auth | Single-operator local deployment is v1 target; adds complexity incompatible with offline-first simplicity |
| LoRaWAN roaming / peering | Cross-network federation; irrelevant for local deployments |
| Embedded MQTT broker | Extension concern, not core LNS |
| Web UI in core binary | Separate process per extension isolation principle |
| JS/Lua payload codec engine | Extension concern; adds runtime weight to core |
| ADR (Adaptive Data Rate) | Requires reliable downlink path and multi-gateway visibility; defer until downlink is proven stable |
| Windows / macOS runtime | Linux-only; community effort if desired |

---

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| CORE-01 | Phase 1 | Pending |
| CORE-02 | Phase 1 | Pending |
| CORE-03 | Phase 5 | Pending |
| CORE-04 | Phase 2 | Pending |
| PROT-01 | Phase 1 | Pending |
| PROT-02 | Phase 1 | Pending |
| PROT-03 | Phase 1 | Pending |
| PROT-04 | Phase 1 | Pending |
| PROT-05 | Phase 1 | Pending |
| PROT-06 | Phase 1 | Pending |
| RELI-01 | Phase 1 | Pending |
| RELI-02 | Phase 1 | Pending |
| RELI-05 | Phase 2 | Pending |
| SEC-01 | Phase 1 | Pending |
| RADIO-01 | Phase 2 | Pending |
| RADIO-02 | Phase 2 | Pending |
| RADIO-03 | Phase 2 | Pending |
| RADIO-04 | Phase 2 | Pending |
| DWNL-01 | Phase 3 | Complete |
| DWNL-02 | Phase 3 | Complete |
| DWNL-03 | Phase 3 | Complete |
| DWNL-04 | Phase 3 | Complete |
| DWNL-05 | Phase 3 | Complete |
| DWNL-06 | Phase 3 | Complete |
| RELI-03 | Phase 4 | Pending |
| RELI-04 | Phase 4 | Pending |
| SEC-02 | Phase 4, Phase 6 (deferred) | Pending |
| DEV-01 | Phase 5, Phase 7 | Pending |
| DEV-02 | Phase 5 | Pending |
| DEV-03 | Phase 5, Phase 7 | Pending |
| DEV-04 | Phase 5 | Pending |
| DEV-05 | Phase 5 | Pending |
| CORE-03 | Phase 5, Phase 7 | Pending |

**Coverage:**
- v1 requirements: 29 total
- Mapped to phases: 29
- Unmapped: 0 ✓

---
*Requirements defined: 2026-04-16*
*Last updated: 2026-04-16 — traceability updated after roadmap creation*
