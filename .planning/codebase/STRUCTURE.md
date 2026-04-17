# Codebase Structure

**Analysis Date:** 2026-04-16

## Directory Layout

```
/home/antonygiomarx/dev/maverick/
├── Cargo.toml                      # Workspace root
├── Cargo.lock                      # Lockfile
├── rustfmt.toml                    # Rust formatter config
├── .cargo/config.toml              # Cargo settings
├── .github/workflows/ci.yml         # CI pipeline
├── AGENTS.md                       # Agent instructions
├── crates/
│   ├── maverick-domain/            # Pure domain types
│   ├── maverick-core/              # Application kernel
│   ├── maverick-extension-contracts/ # Versioned contracts
│   ├── maverick-adapter-radio-udp/ # UDP radio adapter
│   ├── maverick-adapter-radio-spi/ # SPI radio adapter (feature-gated)
│   ├── maverick-adapter-persistence-sqlite/ # SQLite persistence
│   ├── maverick-runtime-edge/      # Edge CLI runtime
│   ├── maverick-cloud-core/         # Cloud hub kernel (future)
│   └── maverick-integration-tests/  # Integration tests
├── .cursor/                        # Cursor IDE rules
│   ├── rules/                      # .mdc rule files
│   └── skills/                     # Rust skill implementations
└── docs/                           # Documentation
```

## Crate Purposes

**`maverick-domain`**
- Purpose: Domain entities and value objects only
- Location: `crates/maverick-domain/`
- Key files: `src/identifiers.rs`, `src/session.rs`, `src/region.rs`
- No I/O, no framework dependencies (only optional `serde`)

**`maverick-core`**
- Purpose: Application kernel (use cases, ports, protocols)
- Location: `crates/maverick-core/`
- Key modules: `use_cases/`, `ports/`, `protocol/`, `storage/`, `health/`, `lns_config.rs`, `error.rs`

**`maverick-extension-contracts`**
- Purpose: Stable v1.x forward-compatible contracts
- Location: `crates/maverick-extension-contracts/`
- Key file: `src/lib.rs` with `SyncBatchEnvelopeV1`

**`maverick-adapter-radio-udp`**
- Purpose: UDP radio transport adapter (GWMP/UDP)
- Location: `crates/maverick-adapter-radio-udp/`
- Key files: `src/gwmp.rs`, `src/gwmp_udp_uplink_source.rs`, `src/resilient.rs`

**`maverick-adapter-radio-spi`**
- Purpose: SX1302/SX1303 SPI concentrator (feature-gated `spi` feature)
- Location: `crates/maverick-adapter-radio-spi/`
- Key files: `src/spi_uplink.rs`, `src/ingress_identity.rs`

**`maverick-adapter-persistence-sqlite`**
- Purpose: SQLite persistence for sessions, uplinks, audit
- Location: `crates/maverick-adapter-persistence-sqlite/`
- Key modules: `persistence/`, `schema.rs`

**`maverick-runtime-edge`**
- Purpose: Edge runtime CLI composition
- Location: `crates/maverick-runtime-edge/`
- Key files: `src/main.rs`, `src/ingest/`, `src/commands/`, `src/probe.rs`

**`maverick-cloud-core`**
- Purpose: Cloud/hub kernel for future ingestion
- Location: `crates/maverick-cloud-core/`
- Note: Edge runtime does not depend on this crate

**`maverick-integration-tests`**
- Purpose: Integration tests
- Location: `crates/maverick-integration-tests/`
- Test command: `cargo test -p maverick-integration-tests`

## Key File Locations

**Entry Points:**
- `crates/maverick-runtime-edge/src/main.rs` - Edge CLI (`maverick-edge` binary)
- `crates/maverick-extension-tui/src/main.rs` - TUI extension binary (separate crate)

**Domain Types:**
- `crates/maverick-domain/src/identifiers.rs` - `DevEui`, `DevAddr`, `GatewayEui` (newtype wrappers)
- `crates/maverick-domain/src/session.rs` - `SessionSnapshot`, `DeviceClass`, `LoRaWANVersion`
- `crates/maverick-domain/src/region.rs` - `RegionId` enum (EU868, US915, AU915, AS923, EU433)

**Core Ports:**
- `crates/maverick-core/src/ports/mod.rs` - Port trait re-exports
- `crates/maverick-core/src/ports/radio_transport.rs` - `RadioTransport`, `UplinkObservation`, `DownlinkFrame`
- `crates/maverick-core/src/ports/uplink_source.rs` - `UplinkSource`, `UplinkReceive`
- `crates/maverick-core/src/ports/uplink_repository.rs` - `UplinkRepository`, `UplinkRecord`
- `crates/maverick-core/src/ports/session_repository.rs` - `SessionRepository`

**Core Use Cases:**
- `crates/maverick-core/src/use_cases/ingest_uplink.rs` - `IngestUplink` service (423 lines with tests)

**Protocol:**
- `crates/maverick-core/src/protocol/capability.rs` - `ProtocolCapability` trait
- `crates/maverick-core/src/protocol/lorawan_10x_class_a.rs` - LoRaWAN 1.0.x Class A implementation

**Storage:**
- `crates/maverick-core/src/storage/policy.rs` - `InstallProfile`, `StoragePolicy`, `RetentionTier`
- `crates/maverick-core/src/storage/pressure.rs` - `StoragePressureSnapshot`, `StoragePressureSource`

**Persistence Adapter:**
- `crates/maverick-adapter-persistence-sqlite/src/lib.rs` - `SqlitePersistence`
- `crates/maverick-adapter-persistence-sqlite/src/persistence/repos.rs` - Port implementations
- `crates/maverick-adapter-persistence-sqlite/src/schema.rs` - SQLite schema

**Configuration:**
- `crates/maverick-core/src/lns_config.rs` - `LnsConfigDocument` (TOML deserialization)

## Naming Conventions

**Crates:**
- `maverick-{category}` - kebab-case

**Modules:**
- `snake_case.rs` per module

**Traits (Ports):**
- Suffix with `Trait` or domain name: `UplinkRepository`, `RadioTransport`
- Async traits use `#[async_trait]` attribute macro

**Types:**
- `PascalCase` for structs, enums: `UplinkObservation`, `ProtocolDecision`
- `PascalCase` for newtype wrappers: `DevAddr`, `DevEui`, `GatewayEui`

**Functions:**
- `snake_case`: `build_b0_uplink`, `compute_mic`, `decrypt_frm_payload`

**Constants:**
- `SCREAMING_SNAKE_CASE`: `DEDUP_WINDOW_MS`

## Where to Add New Code

**New Domain Entity:**
- Add to `crates/maverick-domain/src/`
- Create `new_entity.rs` module, export from `lib.rs`
- No external dependencies

**New Port Trait:**
- Add to `crates/maverick-core/src/ports/`
- Define trait with `#[async_trait]`
- Export from `ports/mod.rs`

**New Use Case:**
- Add to `crates/maverick-core/src/use_cases/`
- Create `new_use_case.rs` implementing port interfaces
- Add unit tests inline

**New Radio Adapter:**
- Create `crates/maverick-adapter-radio-{name}/`
- Implement `UplinkSource`, `RadioTransport` traits from core
- Feature-gate if hardware-specific

**New Persistence Adapter:**
- Create `crates/maverick-adapter-persistence-{name}/`
- Implement all repository traits from core
- Handle storage pressure/retention

**New CLI Command:**
- Add to `crates/maverick-runtime-edge/src/commands.rs` or `commands/`
- Wire through `main.rs` subcommand enum

## Module Structure (lib.rs exports)

**`maverick-domain/src/lib.rs`:**
```rust
pub mod identifiers;  // DevAddr, DevEui, GatewayEui
pub mod region;       // RegionId
pub mod session;      // SessionSnapshot, DeviceClass, LoRaWANVersion
pub use identifiers::{DevAddr, DevEui, GatewayEui};
pub use region::RegionId;
pub use session::{DeviceClass, LoRaWANVersion, SessionSnapshot};
```

**`maverick-core/src/lib.rs`:**
```rust
pub mod error;        // AppError, AppResult
pub mod health;       // ComponentHealth, HealthState, HealthStatus
pub mod lns_config;   // LnsConfigDocument
pub mod ports;        // All port traits
pub mod protocol;     // ProtocolCapability, ProtocolDecision
pub mod storage;      // StoragePolicy, InstallProfile, RetentionTier
pub mod use_cases;    // IngestUplink
```

## Workspace Composition

```
maverick-domain          ← no dependencies
maverick-core            ← depends on maverick-domain
maverick-extension-contracts  ← no dependencies
maverick-adapter-radio-udp     ← depends on maverick-core, maverick-domain
maverick-adapter-radio-spi     ← depends on maverick-core (feature-gated)
maverick-adapter-persistence-sqlite ← depends on maverick-core, maverick-domain
maverick-runtime-edge    ← depends on all adapters, maverick-core
maverick-cloud-core      ← depends on maverick-extension-contracts (edge does NOT depend on this)
maverick-integration-tests ← test dependencies on multiple crates
```

## Special Directories

**`.cursor/rules/`**
- Purpose: Cursor IDE `.mdc` rule files for Rust standards
- Contains: `rust-no-magic-values.mdc`, `rust-clean-code.mdc`, `rust-solid-hexagonal.mdc`, etc.

**`.cursor/skills/`**
- Purpose: Rust-specific skill implementations

**`.github/workflows/`**
- Purpose: CI/CD pipeline (`ci.yml`)

**`docs/`**
- Purpose: Project documentation (e.g., `code-review-checklist.md`)

---

*Structure analysis: 2026-04-16*
