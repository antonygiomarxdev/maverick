# ARCHITECTURE — Maverick Codebase Map
_Generated: 2026-04-16_

## Summary

Maverick is an offline-first, edge-deployed LoRaWAN Network Server (LNS) built on strict hexagonal (ports-and-adapters) architecture. The system receives radio uplinks from LoRa gateways via Semtech GWMP/UDP, validates them through a pluggable protocol capability module, and persists sessions, uplinks, and audit records to a local SQLite database. All I/O dependencies are abstracted behind async traits defined in `maverick-core`; concrete adapters live in separate crates and are wired together only at the composition root (`maverick-runtime-edge`).

---

## Overall Pattern

**Hexagonal Architecture (Ports and Adapters)**

- The inner hexagon contains `maverick-domain` (pure value objects) and `maverick-core` (use cases + port traits).
- The outer ring contains adapter crates (`maverick-adapter-radio-udp`, `maverick-adapter-persistence-sqlite`) that implement the port traits.
- The composition root (`maverick-runtime-edge`) is the only place where adapters are instantiated and wired to use cases. Neither `maverick-domain` nor `maverick-core` import any adapter or I/O crate.
- An optional operator console (`maverick-extension-tui`) shells out to `maverick-edge` rather than linking against adapter crates directly.

---

## Layers

**Layer 1 — Domain (`maverick-domain`)**
- Purpose: Pure value objects and identifiers; zero runtime or I/O dependencies.
- Location: `crates/maverick-domain/src/`
- Contains: `DevEui`, `GatewayEui`, `DevAddr`, `Eui64`, `SessionSnapshot`, `DeviceClass`, `LoRaWANVersion`, `RegionId`.
- Depends on: nothing (serde is an optional feature gate, not a hard dep).
- Used by: `maverick-core`, both adapters, `maverick-runtime-edge`.

**Layer 2 — Application Kernel (`maverick-core`)**
- Purpose: Use cases, port trait definitions, protocol capability modules, storage policy, health types.
- Location: `crates/maverick-core/src/`
- Depends on: `maverick-domain` only.
- Used by: both adapters and the runtime.
- Key sub-modules:
  - `ports/` — all port traits (see Ports section below).
  - `use_cases/ingest_uplink.rs` — the single delivered use case (`IngestUplink`).
  - `protocol/lorawan_10x_class_a.rs` — LoRaWAN 1.0.x Class A policy (stateless; implements `ProtocolCapability`).
  - `lns_config.rs` — deserialization and validation of `/etc/maverick/lns-config.toml`.
  - `storage/` — `StoragePolicy`, `InstallProfile`, `RetentionTier`, `StoragePressureLevel`.
  - `health/` — `HealthStatus`, `ComponentHealth`, `HealthState`.
  - `error.rs` — `AppError` enum (`Domain`, `NotFound`, `InvalidInput`, `Infrastructure`, `CircuitOpen`).

**Layer 3 — Adapters**
- `maverick-adapter-radio-udp` (`crates/maverick-adapter-radio-udp/src/`): Implements inbound GWMP/UDP parsing (no port trait impl yet — produces `UplinkObservation` structs fed to use cases by the runtime); implements `RadioTransport` for UDP downlink with `ResilientRadioTransport` circuit-breaker wrapper.
- `maverick-adapter-persistence-sqlite` (`crates/maverick-adapter-persistence-sqlite/src/`): `SqlitePersistence` implements `SessionRepository`, `UplinkRepository`, `AuditSink`, and `StoragePressureSource`. Also exposes LNS operator tables (`lns_applications`, `lns_devices`, `lns_pending`, `lns_meta`).

**Layer 4 — Composition Root / Runtime (`maverick-runtime-edge`)**
- Purpose: CLI entrypoint; wires adapters to use cases; owns hardware probing and install profile selection.
- Location: `crates/maverick-runtime-edge/src/`
- Binary: `maverick-edge`
- Depends on: `maverick-core`, `maverick-domain`, both adapters.

---

## Ports (Traits in `maverick-core::ports`)

| Port | Trait | Direction | Adapter |
|------|-------|-----------|---------|
| Session persistence | `SessionRepository` | driven | `SqlitePersistence` |
| Uplink persistence | `UplinkRepository` | driven | `SqlitePersistence` |
| Audit log | `AuditSink` | driven | `SqlitePersistence` |
| Radio transport (downlink) | `RadioTransport` | driven | `UdpDownlinkTransport` / `ResilientRadioTransport` |
| Uplink ingress identity | `UplinkIngressBackend` | driving | `GwmpUdpIngressBackend` |
| Downlink queue | `DownlinkRepository` + `DownlinkEnqueue` | driven | (not yet implemented) |
| Device registry | `DeviceRepository` | driven | (not yet implemented) |
| Storage pressure | `StoragePressureSource` | driven | `SqlitePersistence` |

---

## Core Domain Model

**Identifiers** (`maverick-domain::identifiers`):
- `Eui64([u8; 8])` — base EUI type.
- `DevEui(Eui64)` — device 64-bit identifier.
- `GatewayEui(Eui64)` — gateway 64-bit identifier.
- `DevAddr(u32)` — 32-bit device address (assigned after OTAA or set in ABP).

**Session** (`maverick-domain::session`):
- `SessionSnapshot` — minimal session view: `dev_eui`, `dev_addr`, `region`, `class`, `uplink_frame_counter`, `downlink_frame_counter`, `application_id`.
- `DeviceClass` — `ClassA | ClassB | ClassC` (only ClassA active in v1).
- `LoRaWANVersion` — `V1_0_x` only.

**Region** (`maverick-domain::region`):
- `RegionId` — `Eu868 | Us915 | Au915 | As923 | Eu433`.

**Uplink Observation** (`maverick-core::ports::radio_transport`):
- `UplinkObservation` — normalized uplink crossing the adapter→core boundary: gateway EUI, dev_addr, region, f_cnt, f_port, payload bytes, RSSI, SNR.
- `DownlinkFrame` — outbound payload to send via transport adapter.

**Protocol** (`maverick-core::protocol`):
- `ProtocolCapability` trait: `id()`, `supports(version, class, region)`, `validate_uplink(ctx) -> ProtocolDecision`.
- `ProtocolDecision` variants: `Accept`, `RejectNoSession`, `RejectRegionMismatch`, `RejectUnsupportedClass`, `RejectDuplicateFrameCounter`.
- `LoRaWAN10xClassA` — concrete implementation; stateless struct.

---

## Data Flow

**Happy-path uplink ingestion:**

1. A Semtech packet forwarder sends a `PUSH_DATA` UDP datagram to `maverick-edge` on the configured bind address.
2. `ingest/gwmp_loop.rs` (`run_radio_ingest_supervised`) receives the raw bytes.
3. `maverick_adapter_radio_udp::parse_push_data()` parses the GWMP header and JSON `rxpk` array into a `GwmpUplinkBatch` containing one or more `UplinkObservation` values.
4. Each observation is passed to `ingest_uplink_with_lns_guard()` (`ingest/lns_guard.rs`):
   - If a matching session exists in SQLite → proceed to step 5.
   - If no session and autoprovision is enabled → insert a `lns_pending` row and emit an audit event; reject with `AppError::Domain`.
5. `IngestUplink::execute()` (`use_cases/ingest_uplink.rs`) runs:
   a. `SessionRepository::get_by_dev_addr` — fetch session snapshot.
   b. `ProtocolCapability::validate_uplink` — apply LoRaWAN 1.0.x Class A rules (region check, class check, FCnt monotonic check).
   c. On `ProtocolDecision::Accept`: `UplinkRepository::append` → persist uplink record.
   d. `SessionRepository::upsert` → update `uplink_frame_counter`.
   e. `AuditSink::emit` → append `success` audit record.
6. JSON result counters printed to stdout; structured logs via `tracing`.

**LNS config load flow:**

1. Operator runs `maverick-edge config load --config-path /etc/maverick/lns-config.toml`.
2. `commands/config.rs` parses and validates `LnsConfigDocument` (`lns_config.rs`).
3. `SqlitePersistence::lns_load_config()` upserts rows into `lns_applications`, `lns_devices`, `lns_meta`.
4. Subsequent ingest cycles find sessions for those devices.

---

## Resilience Pattern

`ResilientRadioTransport` (`maverick-adapter-radio-udp::resilient`) wraps any `RadioTransport` with:
- Per-attempt timeout (`tokio::time::timeout`).
- Exponential backoff retry loop.
- Three-state circuit breaker: `Closed → Open → HalfOpen → Closed`.
- All state held in `AtomicU32` + `Mutex<CircuitState>` (no external dependency).

---

## Extension / Cloud Sync Surface

`maverick-extension-contracts` defines `SyncBatchEnvelopeV1` and `SyncEventV1` — the wire schema for edge-to-hub store-and-forward replication (not executed in v1 edge runtime). `maverick-cloud-core` defines the `HubSyncIngest` trait. Neither crate is linked by the edge runtime.

---

## Error Handling Strategy

All application-layer errors use `AppError` (from `maverick-core::error`). Port traits return `AppResult<T> = Result<T, AppError>`. Error variants map to:
- `Domain` — business rule violations (bad FCnt, unknown DevAddr).
- `NotFound` — missing entity.
- `InvalidInput` — parse/validation failures.
- `Infrastructure` — SQLite, I/O, task join errors.
- `CircuitOpen` — radio transport circuit open.

Errors from use cases propagate to the composition root; the runtime logs them via `tracing::warn!` and counts them in JSON output counters.

---

## Crate Dependency Graph

```
maverick-domain          (no workspace deps)
    └── maverick-core    (→ maverick-domain)
            ├── maverick-adapter-persistence-sqlite  (→ core, domain)
            ├── maverick-adapter-radio-udp           (→ core, domain)
            └── maverick-runtime-edge                (→ core, domain, both adapters)

maverick-extension-contracts  (no workspace deps)
    └── maverick-cloud-core   (→ extension-contracts)

maverick-extension-tui        (→ maverick-core only; shells out to maverick-edge binary)

maverick-integration-tests    (→ core, domain, extension-contracts, both adapters)
```

Note: `maverick-extension-tui` and `maverick-cloud-core` are **not** linked by `maverick-runtime-edge` — they are entirely separate binaries/libraries.

---

## Gaps / Unknowns

- `DeviceRepository` and `DownlinkRepository`/`DownlinkEnqueue` port traits are defined but have no adapter implementations; downlink scheduling is not yet wired end-to-end.
- OTAA join handling (JoinRequest/JoinAccept exchange) is absent; only ABP and pre-configured OTAA sessions ingested via `config load` are operational.
- `HybridRetentionDefaults` `constrained()`/`balanced()`/`high_capacity()` constructors all return the same struct; the profile differentiation only manifests in `InstallProfile::default_storage_policy()` — the named constructors are vestigial.
- Cloud sync (`SyncBatchEnvelopeV1` / `HubSyncIngest`) is defined but has no runtime trigger or scheduler.
- Region inference in `gwmp.rs::infer_region()` uses overlapping frequency ranges that could produce incorrect `RegionId` for borderline frequencies.
