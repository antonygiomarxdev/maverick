# INTEGRATIONS — Maverick Codebase Map
_Generated: 2026-04-16_

## Summary
Maverick is an offline-first edge LNS (LoRaWAN Network Server). Its only external interface at runtime is a UDP socket that receives Semtech GWMP PUSH_DATA datagrams from LoRaWAN gateways. All state is persisted locally in a bundled SQLite database. There are no third-party cloud APIs, no HTTP clients, and no message brokers wired in the current implementation.

---

## Radio / Gateway Interface (GWMP over UDP)

**Protocol:** Semtech Gateway Message Protocol (GWMP), PUSH_DATA message type only (identifier `0x00`)

**Transport:** UDP, default bind address `0.0.0.0:17000`

**Implementation crate:** `crates/maverick-adapter-radio-udp/`

**Key source files:**
- `crates/maverick-adapter-radio-udp/src/gwmp.rs` — PUSH_DATA binary header parser + `rxpk` JSON body decoder
- `crates/maverick-adapter-radio-udp/src/resilient.rs` — Circuit-breaker / resilience policy wrapper (`ResilientRadioTransport`, `CircuitStateView`, `ResiliencePolicy`)
- `crates/maverick-adapter-radio-udp/src/udp_downlink.rs` — UDP downlink sender (`UdpDownlinkTransport`)
- `crates/maverick-adapter-radio-udp/src/stub.rs` — In-process stub for tests (`UdpRadioStub`)
- `crates/maverick-adapter-radio-udp/src/uplink_ingress.rs` — Ingress backend marker (`GwmpUdpIngressBackend`)

**GWMP datagram layout parsed:**
| Offset | Field | Notes |
|--------|-------|-------|
| 0 | `protocol_version` | u8 |
| 1–2 | token | Not parsed |
| 3 | identifier | Must be `0x00` (PUSH_DATA) |
| 4–11 | gateway EUI | 8-byte big-endian `Eui64` |
| 12+ | JSON body | UTF-8, contains `rxpk[]` array |

**GWMP JSON body fields extracted per `rxpk` entry:**
- `data` — base64-encoded LoRaWAN PHY payload
- `freq` — RF frequency in MHz (used to infer `RegionId`)
- `rssi` — Signal strength (i16, optional)
- `lsnr` — SNR (f32, optional)

**LoRaWAN payload parsing (within `rxpk.data`):**
- Extracts `DevAddr` (bytes 1–4, little-endian), `FCnt` (bytes 6–7, u16→u32), `FPort`, and `FRMPayload`
- MIC (last 4 bytes) is stripped; MIC verification is not yet implemented

**Region inference from frequency:**
| Frequency range (MHz) | Mapped `RegionId` |
|-----------------------|-------------------|
| 863–870 | `Eu868` |
| 902–928 | `Us915` |
| 915–928 | `Au915` |
| 920–925 | `As923` |
| anything else | `Eu868` (fallback) |

**Env vars / CLI flags:**
| Flag | Env var | Default |
|------|---------|---------|
| `--bind` | `MAVERICK_GWMP_BIND` | `0.0.0.0:17000` |
| `--timeout-ms` | `MAVERICK_GWMP_INGEST_TIMEOUT_MS` | `5000` |
| `--read-timeout-ms` | `MAVERICK_GWMP_LOOP_READ_TIMEOUT_MS` | `1000` |
| `--max-messages` | `MAVERICK_GWMP_LOOP_MAX_MESSAGES` | `0` (unlimited) |

---

## Persistence Layer (SQLite)

**Implementation crate:** `crates/maverick-adapter-persistence-sqlite/`

**SQLite client:** `rusqlite 0.33.0` with `features = ["bundled"]` (SQLite compiled into the binary; no system SQLite required)

**Database file:** `maverick.db` (path: `$MAVERICK_DATA_DIR/maverick.db`, default `data/maverick.db`, production `/var/lib/maverick/maverick.db`)

**Schema file:** `crates/maverick-adapter-persistence-sqlite/src/schema.sql`

**PRAGMA settings:**
```sql
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;
PRAGMA synchronous = NORMAL;
```

**Tables:**

| Table | Primary Key | Description |
|-------|-------------|-------------|
| `sessions` | `dev_addr` (INTEGER) | Active LoRaWAN device sessions: EUI, region, class, uplink/downlink frame counters |
| `uplinks` | `id` AUTOINCREMENT | Persisted uplink records: dev_addr, FCnt, raw payload blob, optional application_id |
| `audit_events` | `id` AUTOINCREMENT | Structured audit log: source, operation, entity_type, entity_id, outcome, metadata JSON, timestamp |
| `lns_applications` | `id` TEXT | Declarative LNS application mirror (synced from `lns-config.toml`) |
| `lns_devices` | `dev_eui` BLOB | Device registry: ABP/OTAA mode, keys (app_key, nwk_key, apps_key, nwks_key as BLOBs), enabled flag |
| `lns_pending` | `dev_addr` INTEGER | Auto-provisioned unknown devices pending operator approval |
| `lns_meta` | `id = 1` (singleton) | LNS-wide policy: autoprovision toggle, rate limit, pending TTL |

**Key source files:**
- `crates/maverick-adapter-persistence-sqlite/src/persistence/repos.rs` — repository implementations
- `crates/maverick-adapter-persistence-sqlite/src/persistence/sql.rs` — raw SQL strings
- `crates/maverick-adapter-persistence-sqlite/src/persistence/lns_ops.rs` — LNS config sync operations
- `crates/maverick-adapter-persistence-sqlite/src/schema.rs` — schema initialisation

---

## Declarative Configuration Interface (TOML file)

**File:** `/etc/maverick/lns-config.toml` (default; overridable with `--config-path`)

**Format:** TOML, `schema_version = 1`

**Loaded by:** `maverick-edge config load` command — parses with `toml 0.8.x` crate, upserts into SQLite tables `lns_applications`, `lns_devices`, `lns_meta`

**Key sections:**
- `[autoprovision]` — enable/disable, rate limit per gateway per minute, pending TTL in seconds
- `[[applications]]` — application registry entries (id, name, default_region)
- `[[devices]]` — device entries with `activation_mode = "otaa"` or `"abp"`, keys, region

**Implementation:** `crates/maverick-runtime-edge/src/commands/config.rs` (via `config` subcommand)

---

## LoRaWAN Protocol Engine

**Implementation crate:** `crates/maverick-core/`

**Protocol supported:** LoRaWAN 1.0.x Class A only (v1 baseline)

**Supported regions:** EU868, US915, AU915, AS923, EU433

**Policy module:** `crates/maverick-core/src/protocol/lorawan_10x_class_a.rs` (`LoRaWAN10xClassA` struct implements `ProtocolCapability`)

**Validation rules enforced:**
1. Region must be in the supported set
2. A session must exist for the DevAddr (`RejectNoSession` if missing)
3. Device class must be ClassA
4. Region of session must match observation region
5. Uplink FCnt must be strictly greater than stored counter (32-bit replay protection)

**Not yet implemented:** MIC verification, OTAA join flow, downlink scheduling

---

## Extension / Cloud Sync Contracts

**Crate:** `crates/maverick-extension-contracts/`

- Defines stable sync envelope types used at the edge ↔ cloud boundary
- No active HTTP/MQTT/gRPC client wired; these are data-transfer-object contracts only
- Consumed by `maverick-cloud-core` (`crates/maverick-cloud-core/`) which defines cloud ingestion port traits

---

## Terminal UI (Operator Console)

**Crate:** `crates/maverick-extension-tui/` → binary `maverick-edge-tui` (alias `maverick`)

**External interfaces:**
- Invokes `maverick-edge` as a subprocess (`std::process::Command`) — no IPC socket, no shared memory
- Reads/writes a TOML config file at a user-local path (managed by `crates/maverick-extension-tui/src/config.rs`)
- Reads system memory via `sysinfo 0.30.13` for auto-profile selection

**TUI config file fields:**
- `data_dir` — path to `maverick-edge` data directory
- `gwmp_bind` — UDP bind address forwarded to ingest-loop
- `loop_read_timeout_ms` — forwarded to ingest-loop
- `loop_max_messages` — forwarded to ingest-loop
- `enabled_extensions` — list of active extensions (default: `["console"]`)

---

## No Detected Integrations

The following are **not present** in the codebase:

| Category | Status |
|----------|--------|
| HTTP client (reqwest, hyper, ureq) | Not present |
| MQTT client | Not present |
| gRPC (tonic) | Not present |
| Cloud provider SDKs (AWS, GCP, Azure) | Not present |
| Message broker (Kafka, NATS, RabbitMQ) | Not present |
| Auth provider (OAuth, JWT validation) | Not present |
| Metrics/APM (Prometheus, OpenTelemetry) | Not present |
| Email/SMS/push notifications | Not present |

---

## Environment Variables

| Variable | Crate | Default | Purpose |
|----------|-------|---------|---------|
| `MAVERICK_DATA_DIR` | `maverick-runtime-edge` | `data` | Data directory for SQLite database |
| `MAVERICK_GWMP_BIND` | `maverick-runtime-edge` | `0.0.0.0:17000` | UDP listen address for GWMP ingest loop |
| `MAVERICK_GWMP_INGEST_TIMEOUT_MS` | `maverick-runtime-edge` | `5000` | One-shot ingest wait timeout |
| `MAVERICK_GWMP_LOOP_READ_TIMEOUT_MS` | `maverick-runtime-edge` | `1000` | Supervised loop socket read timeout |
| `MAVERICK_GWMP_LOOP_MAX_MESSAGES` | `maverick-runtime-edge` | `0` | Max datagrams before loop exits (0 = infinite) |
| `RUST_LOG` | `maverick-runtime-edge` | (unset) | `tracing-subscriber` env-filter for log verbosity |

---

## Gaps / Unknowns

- OTAA join procedure (JoinRequest/JoinAccept parsing, session derivation) is not implemented; devices must be pre-provisioned with ABP sessions or approved via the pending flow
- MIC (Message Integrity Code) verification is not implemented — payloads are accepted if FCnt is valid regardless of cryptographic integrity
- Region inference from `rxpk.freq` uses simple range matching and defaults to EU868 on no match; overlapping ranges (AU915 vs US915) may misclassify some frequencies
- `maverick-cloud-core` defines sync ingestion contracts but no cloud transport (HTTP/MQTT) is wired; it is effectively a placeholder crate
- The `maverick-adapter-radio-udp` description says "Semtech-style path to be implemented" — full bi-directional GWMP (PULL_DATA, TX_ACK) is not yet implemented
- No structured log file output; `tracing-subscriber` writes to stderr only; `run_recent_errors` CLI command explicitly notes it is "not yet wired to log file"
