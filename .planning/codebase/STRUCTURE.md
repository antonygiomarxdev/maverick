# STRUCTURE — Maverick Codebase Map
_Generated: 2026-04-16_

## Summary

The workspace is a Cargo workspace of 9 crates under `crates/`, each following a single-responsibility principle aligned to hexagonal architecture layers: domain, application kernel, adapters, runtimes, extensions, and integration tests. All inter-crate versions are managed via `[workspace.dependencies]` in the root `Cargo.toml`. There are two deliverable binaries: `maverick-edge` (the production LNS runtime) and `maverick` / `maverick-edge-tui` (the optional operator console).

---

## Workspace Layout

```
maverick/
├── Cargo.toml                          # Workspace root: members, shared deps, lint baseline
├── Cargo.lock
├── crates/
│   ├── maverick-domain/                # Pure domain value objects — no I/O, no framework
│   ├── maverick-core/                  # Application kernel: use cases, ports, protocol, storage policy
│   ├── maverick-adapter-persistence-sqlite/  # SQLite adapter implementing core ports
│   ├── maverick-adapter-radio-udp/     # GWMP/UDP radio adapter
│   ├── maverick-runtime-edge/          # Binary: maverick-edge CLI + composition root
│   ├── maverick-extension-tui/         # Binary: maverick / maverick-edge-tui operator console
│   ├── maverick-extension-contracts/   # Wire types for future cloud sync (no runtime dep)
│   ├── maverick-cloud-core/            # Hub-side trait for future cloud sync (no runtime dep)
│   └── maverick-integration-tests/     # Integration test harness (dev-only)
├── .planning/                          # Planning and codebase map documents
├── .cursor/skills/                     # Skill definitions (rust-solid-hexagonal, rust-best-practices, etc.)
├── docs/                               # Operator runbooks, ADRs, install guides
├── scripts/                            # install-linux.sh
└── dist/                               # Pre-built release artefacts (pi-preview)
```

---

## Crate Details

### `maverick-domain`
**Purpose:** Foundational value objects; the innermost ring of the hexagon. No I/O, no async, no framework.

```
crates/maverick-domain/src/
├── lib.rs              # Re-exports DevAddr, DevEui, GatewayEui, RegionId, SessionSnapshot
├── identifiers.rs      # Eui64, DevEui, GatewayEui, DevAddr, InvalidEuiHex
├── region.rs           # RegionId enum (Eu868, Us915, Au915, As923, Eu433)
└── session.rs          # SessionSnapshot, DeviceClass, LoRaWANVersion
```

Key types: `DevEui(Eui64)`, `GatewayEui(Eui64)`, `DevAddr(u32)`, `SessionSnapshot`, `DeviceClass`, `LoRaWANVersion`, `RegionId`.

---

### `maverick-core`
**Purpose:** Application kernel. Use cases, all port trait definitions, protocol capability modules, storage policy, health model, LNS config schema. Must not import HTTP, DB, or socket crates.

```
crates/maverick-core/src/
├── lib.rs              # Public API surface; re-exports AppError, HealthState, StoragePolicy, etc.
├── error.rs            # AppError enum + AppResult<T> alias
├── lns_config.rs       # LnsConfigDocument, ApplicationEntry, DeviceEntry, ActivationMode, OtaaKeys, AbpKeys
├── ports/
│   ├── mod.rs          # Re-exports all port traits
│   ├── session_repository.rs    # SessionRepository trait
│   ├── uplink_repository.rs     # UplinkRepository trait + UplinkRecord
│   ├── downlink_repository.rs   # DownlinkRepository + DownlinkEnqueue traits
│   ├── device_repository.rs     # DeviceRepository trait
│   ├── radio_transport.rs       # RadioTransport trait, UplinkObservation, DownlinkFrame
│   ├── uplink_ingress.rs        # UplinkIngressBackend trait + UplinkBackendKind
│   └── audit_sink.rs            # AuditSink trait + AuditRecord
├── protocol/
│   ├── mod.rs                   # Re-exports ProtocolCapability, ProtocolContext, ProtocolDecision
│   ├── capability.rs            # ProtocolCapability trait, ProtocolContext, ProtocolDecision
│   └── lorawan_10x_class_a.rs   # LoRaWAN10xClassA concrete implementation
├── use_cases/
│   ├── mod.rs                   # Re-exports IngestUplink
│   └── ingest_uplink.rs         # IngestUplink struct + execute() + unit tests
├── health/
│   └── mod.rs                   # HealthStatus, ComponentHealth, HealthState
└── storage/
    ├── mod.rs                   # Re-exports StoragePolicy, InstallProfile, RetentionTier, etc.
    ├── policy.rs                # StoragePolicy, InstallProfile, HybridRetentionDefaults, RetentionTier, StoragePressureLevel
    └── pressure.rs              # StoragePressureSnapshot, StoragePressureSource trait
```

Notable: `IngestUplink` holds `Arc<dyn SessionRepository>`, `Arc<dyn UplinkRepository>`, `Arc<dyn AuditSink>`, `Arc<dyn ProtocolCapability>`. It is constructed at the composition root and has no knowledge of adapters.

---

### `maverick-adapter-persistence-sqlite`
**Purpose:** SQLite-backed implementation of `SessionRepository`, `UplinkRepository`, `AuditSink`, `StoragePressureSource`. Also owns the LNS operator tables (`lns_applications`, `lns_devices`, `lns_pending`, `lns_meta`).

```
crates/maverick-adapter-persistence-sqlite/src/
├── lib.rs                          # Re-exports SqlitePersistence, SqlitePersistenceOptions, LNS row types
├── schema.rs                       # SQL query builders (sql_select_session_by_dev_addr, sql_upsert_session, etc.)
├── schema.sql                      # DDL: sessions, uplinks, audit_events, lns_applications, lns_devices, lns_pending, lns_meta
├── persisted_device_class.rs       # PersistedDeviceClassTag: DeviceClass ↔ TEXT column mapping
├── diag.rs                         # Diagnostics helpers
├── limits.rs                       # Numeric constants for retry/backoff
├── sqlite_op.rs                    # SqliteOperation enum for error context
└── persistence/
    ├── mod.rs                      # SqlitePersistence struct + open() + run_blocking()
    ├── repos.rs                    # impl SessionRepository, UplinkRepository, AuditSink, StoragePressureSource
    ├── lns_ops.rs                  # LNS operator methods: lns_autoprovision_policy(), lns_upsert_pending(), lns_load_config(), etc.
    ├── sql.rs                      # init_schema(), map_sqlite(), now_ms(), row_to_session()
    ├── busy.rs                     # run_with_busy_retry() — exponential busy-wait around rusqlite ops
    ├── pressure.rs                 # pressure_snapshot_blocking() — disk ratio calculation
    └── pruning.rs                  # prune_sessions_lru_sql(), prune_uplinks_sql(), prune_audit_sql(), prune_hard_limit_circular_sql()
```

`SqlitePersistence` wraps an `Arc<Inner>` where `Inner` holds a `Mutex<Connection>` (synchronous rusqlite). All async trait impls off-load to `tokio::task::spawn_blocking`.

SQLite schema uses WAL mode, foreign keys ON, synchronous NORMAL. Tables:
- `sessions` (PK: `dev_addr`)
- `uplinks` (autoincrement id)
- `audit_events` (autoincrement id)
- `lns_applications`, `lns_devices`, `lns_pending`, `lns_meta`

---

### `maverick-adapter-radio-udp`
**Purpose:** Semtech GWMP `PUSH_DATA` parser; UDP downlink sender; resilient transport wrapper.

```
crates/maverick-adapter-radio-udp/src/
├── lib.rs              # Re-exports parse_push_data, GwmpUplinkBatch, ResilientRadioTransport, UdpDownlinkTransport, GwmpUdpIngressBackend, UdpRadioStub
├── gwmp.rs             # parse_push_data(), parse_push_data_json(), rxpk_to_observation(), GwmpUplinkBatch, GwmpPacketMeta
├── uplink_ingress.rs   # GwmpUdpIngressBackend — impl UplinkIngressBackend (identity/kind marker)
├── udp_downlink.rs     # UdpDownlinkTransport — impl RadioTransport for UDP send
├── resilient.rs        # ResilientRadioTransport — circuit breaker + retry + backoff wrapper
├── stub.rs             # UdpRadioStub — test/probe no-op transport
└── limits.rs           # Resilience defaults (DEFAULT_MAX_RETRIES, DEFAULT_PER_ATTEMPT_TIMEOUT, etc.)
```

`parse_push_data()` takes a raw UDP datagram `&[u8]` and returns `GwmpUplinkBatch { meta, observations: Vec<UplinkObservation> }`. GWMP header is 12 bytes (version + token + identifier + gateway EUI); JSON body follows.

---

### `maverick-runtime-edge`
**Purpose:** Production binary `maverick-edge`. Only crate that wires adapters to use cases. Owns CLI parsing, hardware probing, install profile selection, and the GWMP ingest loops.

```
crates/maverick-runtime-edge/src/
├── main.rs                 # Cli parser (clap), Commands enum, tokio::main entrypoint
├── cli_constants.rs        # Default bind addr, paths, timeouts, loop limits
├── commands.rs             # run_status(), run_health(), run_setup(), run_probe(), run_storage_policy(), run_storage_pressure(), run_radio_downlink_probe()
├── commands/
│   └── config.rs           # run_config_init(), run_config_validate(), run_config_load(), run_config_show(), run_config_list_*, run_config_approve_device(), run_config_reject_device()
├── edge_json.rs            # JSON output structs: radio_ingest_result(), radio_ingest_loop_result(), RadioIngestCounters
├── ingest/
│   ├── mod.rs              # Re-exports run_radio_ingest_once, run_radio_ingest_supervised
│   ├── gwmp_loop.rs        # Composition root for ingest: opens SQLite, builds IngestUplink, GWMP receive loop
│   └── lns_guard.rs        # ingest_uplink_with_lns_guard(): LNS session gate + autoprovision pending
├── paths.rs                # db_path() helper
├── probe.rs                # HardwareCapabilities::probe(), total_disk_bytes_hint()
└── runtime_capabilities.rs # log_startup_snapshot(): tracing of bind addr + config path at startup
```

Binary name: `maverick-edge`. CLI subcommands: `status`, `setup`, `health`, `recent-errors`, `probe`, `storage-policy`, `storage-pressure`, `radio downlink-probe`, `radio ingest-once`, `radio ingest-loop`, `config init/validate/load/show/list-apps/list-devices/list-pending/approve-device/reject-device`.

---

### `maverick-extension-tui`
**Purpose:** Optional operator console binary (`maverick` / `maverick-edge-tui`). Provides interactive menus and a setup wizard. Does NOT link adapter crates — delegates all LNS operations to the `maverick-edge` subprocess.

```
crates/maverick-extension-tui/src/
├── main.rs             # Cli parser, Commands enum (Setup, ConfigShow, ConfigSet, Status, Health, Doctor, ApplyProfile, StartIngestLoop), fn main()
├── config.rs           # TuiConfig struct + load_or_create_config() + save_config(); persisted in ~/.config/maverick/tui-config.toml
├── console_ui.rs       # print_config() terminal rendering
├── doctor.rs           # run_doctor_dashboard(), probe_edge_capabilities()
├── edge_runner.rs      # run_edge_command(): exec maverick-edge subprocess with env vars
├── ingest_loop.rs      # StartIngestLoop helper (calls run_edge_command)
├── lns_file.rs         # lns-config.toml file helpers for wizard
├── lns_wizard.rs       # Interactive LNS device/application add wizard
├── menu_interactive.rs # Top-level interactive menu loop
├── menu_lorawan.rs     # LoRaWAN-specific submenu
├── profiles.rs         # apply_profile_by_name() — maps auto/constrained/balanced/high-capacity to TuiConfig changes
└── setup_wizard.rs     # run_setup_wizard() / run_setup_non_interactive()
```

`TuiConfig` is stored at `~/.config/maverick/tui-config.toml`. Fields: `data_dir`, `gwmp_bind`, `loop_read_timeout_ms`, `loop_max_messages`, `enabled_extensions`.

---

### `maverick-extension-contracts`
**Purpose:** Shared wire types for edge-to-hub replication. No runtime dependency in v1 edge.

```
crates/maverick-extension-contracts/src/
└── lib.rs    # SyncBatchEnvelopeV1, SyncEventV1; EXTENSION_CONTRACT_VERSION = "1.0.0"
```

---

### `maverick-cloud-core`
**Purpose:** Hub-side port trait for future cloud sync. Not linked by any deployed binary.

```
crates/maverick-cloud-core/src/
└── lib.rs    # HubSyncIngest trait: accept_batch(&SyncBatchEnvelopeV1) -> Result<(), String>
```

---

### `maverick-integration-tests`
**Purpose:** Integration test harness. Links all production crates (except TUI and cloud-core). Tests run against real SQLite in temp files.

```
crates/maverick-integration-tests/
├── src/lib.rs
└── tests/
    ├── operator_local_gateway_e2e.rs     # Full ingest path: GWMP parse → core → SQLite
    ├── persistence_sqlite.rs             # SqlitePersistence port contract tests
    ├── radio_transport_resilience.rs     # ResilientRadioTransport circuit-breaker tests
    └── smoke.rs                          # Minimal sanity checks
```

---

## Key File Locations

**Production entry points:**
- `crates/maverick-runtime-edge/src/main.rs` — `maverick-edge` binary entry point
- `crates/maverick-extension-tui/src/main.rs` — `maverick` / `maverick-edge-tui` binary entry point

**Core use case:**
- `crates/maverick-core/src/use_cases/ingest_uplink.rs` — `IngestUplink::execute()`

**Composition root (adapter wiring):**
- `crates/maverick-runtime-edge/src/ingest/gwmp_loop.rs` — where `SqlitePersistence`, `LoRaWAN10xClassA`, and `IngestUplink` are assembled

**Port definitions:**
- `crates/maverick-core/src/ports/` — all trait definitions

**Database schema:**
- `crates/maverick-adapter-persistence-sqlite/src/schema.sql` — DDL

**LNS config schema:**
- `crates/maverick-core/src/lns_config.rs` — `LnsConfigDocument`

**Storage policy:**
- `crates/maverick-core/src/storage/policy.rs` — `InstallProfile`, `StoragePolicy`

---

## Naming Conventions

**Crates:**
- `maverick-domain` — pure domain layer
- `maverick-core` — application kernel
- `maverick-adapter-{technology}` — hexagonal adapters
- `maverick-runtime-{target}` — composition root + binary
- `maverick-extension-{name}` — optional extensions (TUI, contracts)
- `maverick-cloud-*` — future cloud-side components

**Files:**
- Snake-case `*.rs` matching the primary type they define (e.g., `session_repository.rs` defines `SessionRepository`).
- `mod.rs` used for module entry points with re-exports; avoids deep nesting.
- `limits.rs` in adapters holds numeric constants.

**Types:**
- Traits: `PascalCase` matching port name (e.g., `SessionRepository`, `AuditSink`).
- Enums: PascalCase variants (e.g., `ProtocolDecision::Accept`, `AppError::Domain`).
- Structs holding `Arc<Inner>` for `Clone`-able handle pattern (e.g., `SqlitePersistence`).

---

## Where to Add New Code

**New port (driven adapter boundary):**
1. Define trait in a new file under `crates/maverick-core/src/ports/`.
2. Add `pub use` in `crates/maverick-core/src/ports/mod.rs`.
3. Implement in the appropriate adapter crate.
4. Wire at `crates/maverick-runtime-edge/src/ingest/gwmp_loop.rs` or relevant command handler.

**New use case:**
1. Create `crates/maverick-core/src/use_cases/{name}.rs`.
2. Add `pub mod` and `pub use` in `crates/maverick-core/src/use_cases/mod.rs`.
3. Struct takes `Arc<dyn PortTrait>` dependencies; no adapters directly.

**New CLI subcommand:**
1. Add variant to `Commands` enum in `crates/maverick-runtime-edge/src/main.rs`.
2. Implement handler in `crates/maverick-runtime-edge/src/commands.rs` or a new file under `crates/maverick-runtime-edge/src/commands/`.

**New domain type:**
1. Add to the appropriate module in `crates/maverick-domain/src/`.
2. Re-export from `crates/maverick-domain/src/lib.rs`.

**New integration test:**
1. Add a test file under `crates/maverick-integration-tests/tests/`.
2. Use `SqlitePersistence::open()` with a temp file path for isolation.

---

## Special Directories

**`target/`:**
- Purpose: Cargo build output.
- Generated: Yes. Committed: No.

**`.planning/codebase/`:**
- Purpose: Codebase map documents consumed by GSD planning tools.
- Generated: Yes (by mapper). Committed: Yes (planning artefacts).

**`.cursor/skills/`:**
- Purpose: Project-specific Cursor/Claude skill definitions (`rust-solid-hexagonal`, `rust-best-practices`, `rust-clean-code`, `rust-linter-configuration`, `rust-no-magic-values`).
- Committed: Yes.

**`docs/adr/`:**
- Purpose: Architecture Decision Records.

**`dist/pi-preview/`:**
- Purpose: Pre-built binaries for Raspberry Pi preview releases.

---

## Gaps / Unknowns

- `maverick-cloud-core` and `maverick-extension-contracts` are workspace members but are not consumed by any binary yet; it is unclear whether they will be linked into the edge runtime or remain a separate cloud-side deployment unit.
- `maverick-extension-tui` does not link `maverick-adapter-*` crates — it discovers the `maverick-edge` binary by `PATH` lookup. The exact binary resolution mechanism in `edge_runner.rs` was not confirmed in detail.
- No `Dockerfile` or `docker-compose.yml` was found at workspace root; container build is referenced in CI but the `Dockerfile` location is not in the main tree.
