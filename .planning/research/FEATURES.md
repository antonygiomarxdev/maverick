# FEATURES RESEARCH — Maverick LNS
_Generated: 2026-04-16_

---

## Summary

A production-quality LoRaWAN Network Server must enforce three hard contracts from the spec: cryptographic frame integrity (MIC), 32-bit frame counter continuity, and Class A receive-window scheduling for downlinks. Everything beyond those three is layered capability. For an offline-first, single-operator LNS, the differentiating opportunity is in determinism and operator ergonomics — fast-path reliability, local observability, and an extension boundary that lets community tooling consume data without ever touching the core. The current Maverick codebase has the architectural shape right but is missing the spec-required security layer (MIC verification) and the downlink scheduling path that makes LoRaWAN Class A actually work.

---

## Table Stakes (must have for a real LNS)

These are features without which Maverick cannot be called a LoRaWAN Network Server in any meaningful sense. They are either mandated by LoRaWAN 1.0.x/1.1 spec or universally present in every production LNS (ChirpStack, TTN Stack V3, BasicStation network controller).

### Protocol / Security

- **MIC verification (AES-CMAC over NwkSKey)** — High — Without this, any device can inject arbitrary data by spoofing a known DevAddr with an incrementing FCnt. The LNS spec requires verification of the 4-byte Message Integrity Code in every uplink frame before the payload is accepted. This is the single most critical gap in Maverick today. ChirpStack and TTN both reject frames with invalid MIC before any session lookup; Maverick currently accepts everything.

- **32-bit FCnt reconstruction from 16-bit wire value** — Med — LoRaWAN devices transmit only the 16 low-order bits of FCnt over the air. The LNS must reconstruct the full 32-bit value using the upper 16 bits from session state. Maverick currently stores `uplink_frame_counter` as `u32` but the GWMP parse path only populates the 16-bit wire value. After 65535 frames the session is permanently broken. This is a day-1 production bug.

- **FCnt gap tolerance (configurable miss window)** — Low — LoRaWAN 1.0.x allows a tolerance window (spec default: 16384 frames) for devices that reset FCnt after a reboot while using ABP. Without tolerance, a single device reboot breaks the session permanently. ChirpStack uses a `max_fcnt_gap` per device profile. ABP-only deployments are most exposed; this is operator-critical.

- **Duplicate uplink suppression (per-gateway deduplication)** — Med — When multiple gateways hear the same uplink (common in overlapping coverage areas), the LNS receives duplicate GWMP PUSH_DATA packets. Each duplicate would otherwise be stored as an independent uplink row and trigger downstream processing twice. The LNS must deduplicate within a time window keyed on (DevAddr, FCnt). ChirpStack uses a 200ms deduplication window with best-RSSI gateway selection.

- **Confirmed uplink ACK scheduling (Class A RX1/RX2 downlink windows)** — High — Class A devices open exactly two receive windows after each uplink (RX1 at +1s, RX2 at +2s with fallback data rate/frequency). When a device sends a confirmed uplink (MType = Confirmed Data Up), the LNS must schedule a downlink ACK within the RX1 window or fall back to RX2. Missing both windows means the device will retransmit indefinitely. Maverick has `DownlinkRepository` and `DownlinkFrame` types but zero scheduling logic; the downlink path is entirely unimplemented.

- **Downlink queue per device (at least one pending downlink slot)** — Med — The LNS must buffer downlinks (application payload or MAC commands) to be sent on the next available receive window. LoRaWAN 1.0.x Class A mandates that the device controls when receive windows open; the LNS cannot initiate. Without a downlink queue, there is no way to send any data to devices. The `DownlinkRepository` port trait exists but has no adapter.

- **MAC command handling (core set)** — High — The LNS must respond to at least the mandatory MAC commands from LoRaWAN 1.0.x: `LinkCheckReq` (signal quality answer), `LinkADRReq/Ans` (ADR negotiation), `DevStatusReq/Ans` (battery + SNR), and `RXTimingSetupReq`. These are embedded in the FOpts field of uplink/downlink frames. Without parsing FOpts the LNS silently drops device requests that the spec requires it to answer. ChirpStack has a full MAC command state machine per session; TTN similarly.

- **OTAA Join Request / Join Accept exchange** — High — Over-The-Air Activation is the preferred (and in many deployments, mandatory) device onboarding flow. The LNS must receive JoinRequest (DevEUI + AppEUI + DevNonce), verify the MIC with AppKey, generate a unique DevAddr, derive session keys (NwkSKey, AppSKey), and send a JoinAccept within the RX1/RX2 window. ABP covers existing Maverick deployments but any new sensor hardware defaults to OTAA. This is correctly deferred to v2 in the current roadmap but is a hard requirement before Maverick can onboard any OTAA device.

- **Region-correct downlink channel plan** — Med — When scheduling a downlink in RX1, the LNS must use the correct channel offset and data rate for the region (EU868: same channel as uplink, DR offset; US915: fixed downlink channels 500 kHz subband; AU915/AS923: their own rules). A wrong channel means the device never receives the downlink. Maverick's region types exist but the downlink scheduling layer does not yet enforce channel-plan rules.

- **AppSKey-based payload encryption/decryption** — High — LoRaWAN payloads are AES-128 encrypted with the AppSKey (ABP) or a key derived from AppKey (OTAA). The LNS may optionally forward encrypted payload to an Application Server and let the AS decrypt, or it may decrypt locally. For a local-first LNS with no cloud, local decryption is necessary to make payload data useful. The `abp.apps_key` field exists in the schema but decryption is not applied during ingest.

### Persistence / Reliability

- **Uplink metadata storage (RSSI, SNR, gateway EUI, timestamp, DR, frequency)** — Med — Operators diagnosing coverage gaps need raw radio metadata per uplink, not just payload. The current `uplinks` table stores only `dev_addr`, `f_cnt`, and `payload`. RSSI and SNR are captured in `UplinkObservation` but not persisted. Gateway EUI (which heard the frame) and the receive timestamp are discarded at the persistence layer.

- **Session persistence across restart** — Low — Already implemented in Maverick via SQLite. Mentioned here only because it is universally required; losing sessions on restart means every ABP device appears unknown after a reboot.

- **Audit log with structured outcomes** — Low — Already implemented. Required for operator debugging (which frames were accepted/rejected and why).

### Observability

- **Gateway status tracking (last heartbeat, online/offline)** — Med — Semtech GWMP sends periodic PUSH_STAT packets from the packet forwarder containing gateway GPS coordinates, uplink counts, and status. The LNS should track per-gateway health and expose it to operators. Maverick's UDP adapter receives GWMP but does not store or surface gateway stat packets. ChirpStack stores gateway last-seen timestamp and exposes it via API.

- **Per-device uplink rate / last-seen reporting** — Low — Operators need to know which devices are active, when they last transmitted, and whether frame counter gaps suggest coverage problems. The current schema has no `last_seen_at` on sessions or devices.

---

## Differentiators (competitive advantage for offline-first)

These features are not universally present or are significantly better in an offline-first model than in cloud-first LNS products.

- **Zero-dependency single binary deployment** — Low — Maverick runs as a single static binary with bundled SQLite; no Docker, no Postgres, no Redis, no MQTT broker required. ChirpStack requires PostgreSQL + Redis + optionally MQTT; TTN Stack requires CockroachDB/Postgres. This is a significant advantage for Raspberry Pi or isolated industrial deployments. Already largely achieved; keep protecting this.

- **Autoprovision-and-hold queue for unknown devices** — Med — When a frame arrives from an unknown DevAddr, instead of silently dropping it, Maverick records it in `lns_pending` so the operator can inspect and approve it. ChirpStack and TTN have no equivalent; they simply reject unknown devices. This is unique and extremely useful in field deployments where a technician installs a sensor before configuring the LNS. Already scaffolded in the codebase; make it a first-class operator workflow.

- **Declarative TOML config as source of truth** — Low — Operators can manage devices via a plain text file under version control rather than a web UI or database mutation. This matches how infrastructure-as-code teams work. The `lns-config.toml` format already exists; it needs stability guarantees and good documentation.

- **Offline-resilient extension IPC (store-and-forward to extensions)** — High — When the LNS cannot reach an extension process (HTTP, MQTT bridge, cloud sync), it must buffer events locally rather than blocking ingest or losing data. The core isolation principle (extensions as separate processes) is already set; the IPC surface needs a durable outbound queue (likely SQLite-backed) so that a crashed extension doesn't lose uplinks. ChirpStack uses a Redis/NATS event bus which fails if those services are down. Maverick's offline-first commitment requires a local durable queue alternative.

- **SPI direct radio adapter (no packet forwarder dependency)** — High — For Raspberry Pi + RAK concentrator deployments, eliminating the Semtech packet forwarder removes one process, one config file, and one failure point. The SX1302/SX1303 HAL can be driven directly from Rust via SPI. BasicStation implements this model (Basics Station protocol). This is already listed as active in the roadmap and is a strong differentiator for sub-community deployments that don't want to run `lora_pkt_fwd`.

- **Process supervision with automatic restart** — Low — The LNS core must restart itself on panic without operator intervention. `systemd` handles this at the OS level but Maverick should also self-supervise its internal ingest loop (already partially implemented via `run_radio_ingest_supervised`). This is more mature than most hobbyist LNS options which simply crash and stay down.

- **Hardware compatibility registry (community-curated)** — Low — A machine-readable list of tested hardware (gateway concentrators, SBC models) with verified/untested/unsupported status tags gives operators confidence before buying hardware. No other local LNS has this. Mentioned in the roadmap; it differentiates Maverick as a community project, not just software.

- **Structured JSON output for scripting** — Low — `maverick-edge` already emits JSON counters on stdout. Operators running Maverick headlessly (no TUI) can pipe output to `jq`, monitoring scripts, or log collectors without parsing prose. Preserve and extend this.

- **TUI device management without web server** — Med — ChirpStack, TTN, and BasicStation all require a running web server and browser to manage devices. The Maverick TUI works over SSH on a headless Raspberry Pi with no HTTP port open. For air-gapped or remote deployments this is a meaningful ergonomic advantage. The TUI device management (add/edit/remove via terminal) is in the active roadmap.

---

## Anti-Features (deliberately not build)

- **Multi-tenant device namespacing** — The v1 target is a single operator. Adding tenant isolation (separate key namespaces, per-tenant session tables, access control) adds schema complexity that doesn't benefit a local deployment and creates a maintenance surface. ChirpStack supports multi-tenancy; Maverick should not try to compete on that dimension.

- **Built-in MQTT broker** — Many LNS products embed or tightly couple to MQTT (TTN Stack, ChirpStack Application Server). Running a broker in-process couples the core to a protocol Maverick shouldn't own. MQTT output is an extension concern (separate process). Never put broker logic in the core.

- **Web UI in the LNS core** — Opening an HTTP port in the `maverick-edge` binary violates the extension isolation principle and adds TLS/auth complexity to the core. The web dashboard is correctly deferred to a v2 extension process.

- **Built-in payload codec runtime (JS/Lua engine)** — Embedding a scripting runtime (like ChirpStack's gRPC codec service or TTN's V8 JS sandbox) in the core adds significant binary size, memory pressure, and attack surface. Payload decoding belongs in an extension. On a 512 MB Raspberry Pi this is especially important.

- **Roaming / network peering (LoRaWAN Backend Interfaces)** — The LoRa Alliance Backend Interfaces spec defines fNS/hNS/sNS peering for roaming between network operators. This is irrelevant for a local, single-operator deployment and introduces significant protocol complexity (HTTP/REST peering, device context handoff). Never in scope for Maverick.

- **LoRaWAN 1.1 dual-key session complexity in v1** — LoRaWAN 1.1 splits the single NwkSKey into FNwkSIntKey + SNwkSIntKey + NwkSEncKey and adds the NwkKey/AppKey split. The added MIC computation complexity (two-key MIC for 1.1 confirmed frames) is only necessary for 1.1 devices. All practical ABP deployments on LoRaWAN 1.0.x. Implement full 1.0.x security correctly first; 1.1 can be a later protocol module.

- **ADR server (adaptive data rate optimization)** — ADR requires collecting SNR history across multiple uplinks, computing link margin, and sending LinkADRReq MAC commands to adjust spreading factor and TX power. This is valuable but depends on downlink scheduling being solid first. Do not implement ADR until the downlink path is end-to-end working. Building ADR on top of a broken downlink scheduler leads to devices getting stuck at wrong data rates with no recovery path.

---

## ChirpStack / TTN Reference

**What ChirpStack (v4) does that Maverick should study:**

- ChirpStack splits its responsibilities across `chirpstack` (network server), `chirpstack-gateway-bridge` (GWMP/BasicStation adapter), and application integrations. Maverick's equivalent split is `maverick-edge` (core), `maverick-adapter-radio-udp` (GWMP bridge), and planned extension processes. The layering maps well; don't collapse it.

- ChirpStack's device profile model (separate profile entity containing LoRaWAN version, class, region parameters, ADR config, codec reference) is more ergonomic than Maverick's current flat `DeviceEntry`. For v2, a device profile layer would let operators reuse config across many devices. Not v1 complexity.

- ChirpStack deduplicates uplinks with a 200ms collection window, selects the gateway with best RSSI/SNR for downlink scheduling, and stores all receiving gateways for the frame (for coverage analysis). Maverick should store the best-RSSI gateway at minimum and add per-gateway uplink metadata even before full multi-gateway deduplication is implemented.

- ChirpStack's event system (uplink, join, ack, error events per device) via MQTT/HTTP/Kafka is what extensions consume. Maverick's extension IPC should expose equivalent event types over its local API so extensions can subscribe to the same semantic events without coupling to the DB schema.

- TTN Stack V3 (The Things Stack) requires CockroachDB in production and Redis for session caching — both entirely inappropriate for edge. TTN's join server separation (JS as a separate service) is architecturally correct for global deployments but overengineered for local. Maverick rightly keeps join handling in-process (when implemented).

- BasicStation (Semtech's reference gateway software) uses the Basics Station LNS protocol (WebSocket-based, replacing GWMP for station-side) to talk to the network controller. The Basics Station LNS protocol is increasingly the standard for newer concentrator hardware. Maverick should plan a `maverick-adapter-radio-basicstation` alongside the existing UDP adapter; this is not v1 scope but the hexagonal architecture makes it straightforward to add.

**Key lesson from ChirpStack's architecture:** The most reliable design is a stateless ingest path that validates and persists as fast as possible, then triggers async downstream processing (MAC commands, downlink scheduling, event emission) separately. The ingest hot path must never block on extension delivery. Maverick's current design already follows this — protect it.

---

## Gaps / Unknowns

1. **MIC verification key source**: Once MIC verification is added, the NwkSKey must be retrieved before (or alongside) the session lookup. The current `SessionSnapshot` does not carry key material. The `lns_devices` table stores `nwks_key` but the session fetch path (`get_by_dev_addr`) returns only `SessionSnapshot` which has no key fields. Verify whether key material should be added to `SessionSnapshot` or fetched as a separate query in the ingest use case. Architectural decision needed before implementing MIC.

2. **Downlink timing precision**: Class A RX1 window opens exactly 1 second after the uplink GWMP timestamp. The GWMP `PUSH_DATA` includes a `tmst` field (microsecond counter from gateway). Whether the gateway's `tmst` counter is reliable enough for RX1 scheduling over UDP (vs. BasicStation's GPS-disciplined timing) needs investigation. UDP round-trip jitter could cause missed RX1 windows. This is likely why production deployments prefer BasicStation for downlink-heavy use cases.

3. **FCnt gap tolerance policy for ABP**: The LoRaWAN 1.0.x spec allows `MAX_FCNT_GAP` (default 16384) as an implementation parameter. If the gap is too tight (e.g., 1), a single device reboot with ABP permanently breaks the session. If too loose, a replay attack window opens. The right default for a single-operator local deployment needs a decision; ChirpStack exposes this per-device-profile, TTN hard-codes it.

4. **Uplink deduplication window duration**: The right deduplication window depends on gateway density. A single-gateway deployment (common for Maverick's target) has no duplicates to suppress. A small community deployment with 2-3 gateways needs 100-300ms. Configuring this per-deployment (or auto-detecting single-gateway mode) avoids unnecessary latency in the common case.

5. **AppSKey management surface**: For local payload decryption, AppSKey must be stored and used. Currently `abp.apps_key` is stored in `lns_devices` but not fetched during ingest. The question is whether decrypted payload should be stored alongside encrypted payload or replace it. Storing both is safest (raw bytes for audit, decoded for application use) but doubles storage for the payload column.

6. **Extension IPC protocol choice**: The extension isolation principle mandates a local API. Whether that API is a Unix socket + line-delimited JSON, a local HTTP/REST endpoint, or a SQLite-based outbox (extensions poll the DB directly) has significant implications for extension development complexity. The SQLite outbox model (extensions read from a `lns_events` table) requires no IPC protocol work and leverages existing infrastructure; it should be evaluated seriously before committing to a socket/HTTP API.
