# Architecture

**Analysis Date:** 2026-04-16

## Pattern Overview

**Overall:** Hexagonal (Ports & Adapters) with Clean Architecture layers

**Key Characteristics:**
- Domain layer is completely isolated from infrastructure (no I/O dependencies)
- Core defines port traits; adapters provide concrete implementations
- Dependency flow points inward toward domain
- Async trait pattern (`async_trait`) for async port interfaces

## Layers

**Domain Layer:**
- Purpose: Pure business entities, value objects, and domain logic with zero external dependencies
- Location: `crates/maverick-domain/src/`
- Contains: Identifiers (`DevEui`, `DevAddr`, `GatewayEui`), `RegionId`, `SessionSnapshot`, `DeviceClass`, `LoRaWANVersion`
- Depends on: Nothing (only optional `serde` feature flag)
- Used by: `maverick-core` (application kernel)

**Application Kernel (Core):**
- Purpose: Use cases, port traits (interfaces), protocol capability modules, storage policy
- Location: `crates/maverick-core/src/`
- Contains:
  - `use_cases/` - `IngestUplink` service
  - `ports/` - Trait definitions for all infrastructure concerns
  - `protocol/` - `ProtocolCapability` trait + `LoRaWAN10xClassA` implementation
  - `storage/` - `StoragePolicy`, `InstallProfile`, retention tiers
  - `lns_config.rs` - Declarative TOML configuration types
  - `health/` - `HealthState`, `ComponentHealth`
  - `error.rs` - `AppError` enum with domain/not-found/infrastructure variants
- Depends on: `maverick-domain` only
- Used by: Adapters, runtime crates

**Adapters (Infrastructure):**
- Purpose: Concrete implementations of core ports
- Location: `crates/maverick-adapter-*/src/`
- Types:
  - `maverick-adapter-radio-udp` - GWMP/UDP radio transport
  - `maverick-adapter-radio-spi` - SX1302/SX1303 SPI concentrator (feature-gated `spi` feature)
  - `maverick-adapter-persistence-sqlite` - SQLite persistence for sessions, uplinks, audit
- Depends on: `maverick-core`, `maverick-domain`
- Used by: Runtime composition

**Runtime (Application Assembly):**
- Purpose: CLI entry point, dependency wiring, runtime loop orchestration
- Location: `crates/maverick-runtime-edge/src/main.rs` + supporting modules
- Contains: Command handlers, ingest loop, hardware probing, config management
- Depends on: All adapters, `maverick-core`

**Extension Contracts:**
- Purpose: Versioned contracts for future sync and edge-to-cloud integration
- Location: `crates/maverick-extension-contracts/src/lib.rs`
- Contains: `SyncBatchEnvelopeV1`, `SyncEventV1` (v1.0.0 schema)
- Used by: Future `maverick-cloud-core` hub implementation

**Cloud Core:**
- Purpose: Hub-side ports for future ingestion (edge does not depend on this)
- Location: `crates/maverick-cloud-core/src/lib.rs`
- Contains: `HubSyncIngest` trait

## Data Flow

**Uplink Ingest Flow:**

1. **Radio Adapter** (`maverick-adapter-radio-udp` or `maverick-adapter-radio-spi`)
   - Listens on UDP socket or SPI bus
   - Parses GWMP packets into `UplinkObservation`
   - Implements `UplinkSource` port trait

2. **Core Use Case** (`IngestUplink` in `crates/maverick-core/src/use_cases/ingest_uplink.rs`)
   - Receives `UplinkObservation`
   - Fetches session via `SessionRepository`
   - Reconstructs 32-bit FCnt from 16-bit wire value
   - Validates via `ProtocolCapability` (`LoRaWAN10xClassA`)
   - Verifies MIC using NwkSKey
   - Decrypts payload using AppSKey
   - Checks duplicate via `UplinkRepository`
   - Persists uplink record
   - Updates session frame counter
   - Emits audit record via `AuditSink`

3. **Persistence Adapter** (`maverick-adapter-persistence-sqlite`)
   - Implements `UplinkRepository`, `SessionRepository`, `AuditSink` traits
   - Stores data in SQLite with hybrid retention policy

## Key Abstractions

**Ports (Traits in `maverick-core/src/ports/`):**

| Port | Purpose | Key Methods |
|------|---------|-------------|
| `RadioTransport` | Downlink frame dispatch | `send_downlink()` |
| `UplinkSource` | Radio-agnostic uplink ingestion | `next_batch()` → `UplinkReceive` |
| `UplinkRepository` | Uplink persistence | `append()`, `is_duplicate()` |
| `SessionRepository` | Device session storage | `get_by_dev_addr()`, `upsert()` |
| `DeviceRepository` | Device registry | (interface defined) |
| `DownlinkRepository` | Downlink queue | (interface defined) |
| `AuditSink` | Operational audit trail | `emit(AuditRecord)` |
| `UplinkIngressBackend` | Protocol-specific packet parsing | Backend identification |

**Protocol Capability Pattern:**
```rust
// crates/maverick-core/src/protocol/capability.rs
pub trait ProtocolCapability: Send + Sync {
    fn id(&self) -> &'static str;
    fn supports(&self, version: LoRaWANVersion, class: DeviceClass, region: RegionId) -> bool;
    fn validate_uplink(&self, ctx: ProtocolContext<'_>) -> AppResult<ProtocolDecision>;
}
```

**Storage Policy Pattern:**
```rust
// crates/maverick-core/src/storage/policy.rs
pub enum InstallProfile { Constrained, Balanced, HighCapacity }
pub enum RetentionTier { Critical, Operational, Telemetry }
pub struct StoragePolicy {
    pub circular_at_hard_limit: bool,
    pub elevated_use_ratio: f32,
    pub critical_use_ratio: f32,
    pub max_records_telemetry: u64,
    pub max_records_operational: u64,
    pub max_records_critical: u64,
}
```

## Entry Points

**CLI Binary:**
- Location: `crates/maverick-runtime-edge/src/main.rs`
- Crate: `maverick-runtime-edge`
- Binary name: `maverick-edge`
- Subcommands: `status`, `setup`, `health`, `probe`, `storage-policy`, `storage-pressure`, `radio`, `config`

**Ingest Loop:**
- Location: `crates/maverick-runtime-edge/src/ingest/gwmp_loop.rs`
- Functions: `run_radio_ingest_once()`, `run_radio_ingest_supervised()`

## Error Handling

**Strategy:** Typed error enum (`AppError`) with domain-specific variants

```rust
// crates/maverick-core/src/error.rs
pub enum AppError {
    Domain(String),        // Business rule violations
    NotFound(String),      // Entity not found
    InvalidInput(String), // Input validation
    Infrastructure(String),// I/O, DB, network
    CircuitOpen(String),   // Resilience circuit breaker
}
```

**No raw panics in core.** Core returns `AppResult<T>` for all fallible operations.

## Cross-Cutting Concerns

**Logging:** `tracing` crate with structured fields; no raw `println!`

**Async:** All port traits use `async_trait`; runtime uses `tokio` with multi-thread runtime

**Serialization:** `serde` for config and persistence; JSON for CLI output, TOML for config files

**Resilience:** Circuit breaker pattern in `maverick-adapter-radio-udp/src/resilient.rs` (`ResilientRadioTransport`)

**Validation:** LNS config validates hex fields, region IDs, OTAA/ABP requirements before DB operations

---

*Architecture analysis: 2026-04-16*
