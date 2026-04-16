# CONVENTIONS — Maverick Codebase Map
_Generated: 2026-04-16_

## Summary

Maverick is a Rust workspace using hexagonal architecture: a framework-free `maverick-core` domain/port layer, adapter crates implementing those ports, and runtime crates composing them. Naming follows standard Rust idioms (snake_case modules/functions, PascalCase types), error handling is centralized around a single `thiserror`-derived `AppError` enum, async work uses Tokio with `async-trait` for dyn dispatch, and structured tracing is used (never `println!` on the hot path).

---

## Naming Conventions

### Crates
Kebab-case with a `maverick-` prefix and a tier suffix describing role:
- `maverick-domain` — pure value objects / entities
- `maverick-core` — use cases + port traits (no I/O)
- `maverick-adapter-*` — port implementations (e.g., `maverick-adapter-persistence-sqlite`, `maverick-adapter-radio-udp`)
- `maverick-runtime-*` — composition roots / binaries (e.g., `maverick-runtime-edge`)
- `maverick-extension-*` — optional operator tooling (e.g., `maverick-extension-tui`)
- `maverick-integration-tests` — cross-crate integration test harness

### Files
Snake_case. Modules that group subtopics use `mod.rs` + peer files:
```
crates/maverick-adapter-persistence-sqlite/src/persistence/
    mod.rs        ← composition root, SqlitePersistence struct
    repos.rs      ← port trait impls (SessionRepository, UplinkRepository, AuditSink)
    sql.rs        ← SQL helpers
    busy.rs       ← busy-retry logic
    pruning.rs    ← retention pruning
    pressure.rs   ← storage pressure
    lns_ops.rs    ← LNS-specific DB ops
```

### Types
PascalCase structs, enums, traits. Acronyms are title-cased when they lead a word:
- `AppError`, `AppResult<T>` — application-level error and result alias
- `SqlitePersistence`, `UdpDownlinkTransport`, `GwmpUplinkBatch`
- `LoRaWAN10xClassA` — acronym kept as-is when it matches the protocol spec name
- `DevAddr`, `DevEui`, `GatewayEui` — domain value objects as newtype structs

### Functions and methods
Snake_case. Command-handler functions in runtime crates are named `run_<verb>_<noun>`:
- `run_radio_ingest_once`, `run_radio_ingest_supervised`, `run_health`, `run_probe`
- `run_config_init`, `run_config_load`, `run_config_approve_device`

Helper constructors use `open`, `new`, or `bind_*`:
- `SqlitePersistence::open(path, policy, options)`
- `ResilientRadioTransport::new(inner, policy)`
- `UdpDownlinkTransport::bind_ephemeral(addr)`

### Constants
`SCREAMING_SNAKE_CASE` in dedicated `cli_constants.rs` / `limits.rs` files:
```rust
// crates/maverick-runtime-edge/src/cli_constants.rs
DEFAULT_DATA_DIR, DEFAULT_GWMP_BIND_ADDR, EDGE_DB_FILENAME
// crates/maverick-adapter-radio-udp/src/limits.rs
DEFAULT_BACKOFF_BASE, DEFAULT_MAX_RETRIES, DEFAULT_PER_ATTEMPT_TIMEOUT
```

### Enum variants
PascalCase with semantic names that include the decision/state they represent:
```rust
pub enum ProtocolDecision { Accept, RejectNoSession, RejectDuplicateFrameCounter, RejectRegionMismatch, RejectUnsupportedClass }
pub enum AppError { Domain(String), NotFound(String), InvalidInput(String), Infrastructure(String), CircuitOpen(String) }
pub enum CircuitStateView { Closed, Open, HalfOpen }
```

---

## Error Handling

### Central error type
All fallible core/adapter code returns `AppResult<T>` — a type alias for `Result<T, AppError>`:
```rust
// crates/maverick-core/src/error.rs
#[derive(Debug, Error)]
pub enum AppError {
    #[error("domain constraint: {0}")]
    Domain(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("infrastructure: {0}")]
    Infrastructure(String),
    #[error("circuit open: {0}")]
    CircuitOpen(String),
}
pub type AppResult<T> = Result<T, AppError>;
```

### `thiserror` — domain crate only
`thiserror` derives `Error` on `AppError`. No `anyhow` in this codebase.

### Error construction style
String messages are formatted inline at the construction site:
```rust
AppError::Infrastructure(format!("create data dir {}: {e}", parent.display()))
AppError::InvalidInput(format!("gwmp rxpk data base64: {e}"))
AppError::Domain(format!("uplink rejected: {other:?}"))
```

### `map_err` adapter pattern
SQLite errors are mapped through a helper in `sql.rs`:
```rust
.map_err(|e| map_sqlite(SqliteOperation::Open, e))
```

### `?` propagation
All async trait methods propagate with `?`; errors bubble up to command handlers which print JSON:
```rust
// ingest loop handler
Err(e) => {
    failed += 1;
    tracing::warn!(error = %e, "ingest observation failed");
}
```

### Panic policy
`panic = "abort"` in release profile. `unwrap()` is permitted only in tests and `expect()` is preferred over `unwrap()` when a message aids debugging. The workspace lint `#[deny(unused_must_use)]` enforces that `AppResult` is never silently dropped.

---

## Async Patterns

### Runtime
Tokio with `features = ["full"]`. `#[tokio::main]` on the binary entrypoints:
```rust
// crates/maverick-runtime-edge/src/main.rs
#[tokio::main]
async fn main() { ... }
```

### `async-trait`
Every port trait that requires async uses `#[async_trait]` from the `async-trait` crate (workspace dependency):
```rust
#[async_trait]
pub trait SessionRepository: Send + Sync {
    async fn get_by_dev_addr(&self, dev_addr: DevAddr) -> AppResult<Option<SessionSnapshot>>;
    async fn upsert(&self, session: &SessionSnapshot) -> AppResult<()>;
}
```

### Blocking work on a thread-pool
SQLite operations use `tokio::task::spawn_blocking` to avoid blocking the async executor:
```rust
// crates/maverick-adapter-persistence-sqlite/src/persistence/mod.rs
async fn run_blocking<T: Send + 'static>(
    &self,
    f: impl FnOnce(&SqlitePersistence) -> AppResult<T> + Send + 'static,
) -> AppResult<T> {
    let this = self.clone();
    tokio::task::spawn_blocking(move || f(&this))
        .await
        .map_err(|e| AppError::Infrastructure(format!("join blocking task: {e}")))
}
```

### Timeout pattern
`tokio::time::timeout` wraps inner transport calls; elapsed is mapped to an `Infrastructure` error:
```rust
match tokio::time::timeout(timeout_d, async move { inner.send_downlink(&frame).await }).await {
    Ok(Ok(())) => { ... }
    Err(_elapsed) => { last_err = Some(AppError::Infrastructure(format!("radio transport timeout after {} ms", ...))); }
}
```

### Mutex in async context
`tokio::sync::Mutex` is used for async-safe shared state in tests and adapters. `std::sync::Mutex` guards the SQLite connection (held only within blocking closures).

---

## Logging and Tracing

### Crate
`tracing` (workspace dep) for structured logging; `tracing-subscriber` for initialization in the runtime binary.

### Initialization
Only the top-level binary (`maverick-runtime-edge/src/main.rs`) initializes the subscriber:
```rust
tracing_subscriber::fmt()
    .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
    .init();
```
Filter level via `RUST_LOG` env var at runtime.

### Usage style
Structured fields, not interpolated strings:
```rust
// crates/maverick-runtime-edge/src/runtime_capabilities.rs
tracing::info!(
    snapshot_version = report.capability_snapshot.snapshot_version,
    snapshot_id_ms   = report.capability_snapshot.snapshot_id_ms,
    backend_id       = report.capability_snapshot.backend_id,
    listen_bind      = %report.capability_snapshot.listen_bind,
    "ingest capability snapshot (startup)"
);
```

Warn-level for recoverable per-uplink failures:
```rust
tracing::warn!(error = %e, "ingest observation failed");
```

Info-level for backend startup identity:
```rust
tracing::info!(
    backend_id = backend.id(),
    backend_kind = ?backend.kind(),
    "uplink ingress backend (GWMP/UDP)"
);
```

### No `println!` on the hot path
`println!` is reserved for CLI command outputs that intentionally produce machine-readable JSON (e.g., `run_radio_ingest_result` emitting JSON for `maverick-edge` subcommand consumers). Diagnostics always use `tracing`.

---

## Module Organization

### Layer isolation rule
`maverick-domain` has zero dependencies on other workspace crates. `maverick-core` depends only on `maverick-domain`. Adapters depend on `maverick-core` + `maverick-domain`. Runtime crates wire everything together.

### Port trait pattern (hexagonal architecture)
Each port trait lives in its own file under `crates/maverick-core/src/ports/`:
```
ports/
    mod.rs                 ← re-exports everything
    session_repository.rs  ← SessionRepository trait
    uplink_repository.rs   ← UplinkRepository + UplinkRecord
    uplink_ingress.rs      ← UplinkIngressBackend + UplinkBackendKind
    radio_transport.rs     ← RadioTransport + UplinkObservation + DownlinkFrame
    audit_sink.rs          ← AuditSink + AuditRecord
    downlink_repository.rs ← DownlinkRepository + DownlinkEnqueue
```

`mod.rs` re-exports all public items with `pub use`:
```rust
pub use session_repository::SessionRepository;
pub use uplink_repository::{UplinkRecord, UplinkRepository};
```

### Use-case files
Each use case is a struct with an `execute` method:
```rust
pub struct IngestUplink {
    pub sessions: Arc<dyn SessionRepository>,
    pub uplinks: Arc<dyn UplinkRepository>,
    pub audit: Arc<dyn AuditSink>,
    pub protocol: Arc<dyn ProtocolCapability>,
}
impl IngestUplink {
    pub async fn execute(&self, obs: UplinkObservation) -> AppResult<()> { ... }
}
```
No service traits are defined for use cases themselves — they are concrete structs with injected trait object dependencies.

### In-module unit tests
`#[cfg(test)]` test modules live at the bottom of the same file they test:
```rust
// bottom of crates/maverick-core/src/use_cases/ingest_uplink.rs
#[cfg(test)]
mod tests {
    use super::*;
    // in-memory stub implementations + #[tokio::test] cases
}
```

---

## Code Style Rules

### Workspace lint policy (`Cargo.toml`)
```toml
[workspace.lints.rust]
unused_must_use = "deny"
unsafe_op_in_unsafe_fn = "warn"

[workspace.lints.clippy]
dbg_macro = "deny"
todo = "warn"
unimplemented = "warn"
```

### `#[allow(...)]` usage
Sparingly. One known usage: `#[allow(non_camel_case_types)]` on `LoRaWANVersion::V1_0_x` to match protocol spec naming.

### Serde attributes
`#[serde(rename_all = "snake_case")]` on enums that cross JSON boundaries. `#[serde(default)]` on optional/vec fields for forward compat:
```rust
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UplinkBackendKind { GwmpUdp }

pub struct LnsConfigDocument {
    #[serde(default)]
    pub applications: Vec<ApplicationEntry>,
}
```

### Feature-gated `serde` in domain crate
`maverick-domain` gates serde derives behind a `serde` feature:
```rust
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SessionSnapshot { ... }
```

### Struct literal constants named with underscores for readability
```rust
DevAddr(0xAB_CD_00_01)
DevAddr(0x0403_0201)
```

### Clap CLI pattern
`#[derive(Parser)]` on `Cli`, `#[derive(Subcommand)]` on `Commands`. `#[command(about = "...")]` and `#[arg(...)]` annotations inline. Subcommand dispatch via `match cli.command { ... }`.

---

## Gaps / Unknowns

- No `rustfmt.toml` found — default `rustfmt` settings are assumed but not enforced via checked-in config.
- No CI configuration was read; unclear if `cargo clippy -- -D warnings` is enforced in CI.
- `maverick-extension-tui` (`main.rs`) has a synchronous `main() -> Result<(), String>` rather than `#[tokio::main]` — consistent with its role as a synchronous subprocess orchestrator, but worth noting as an exception.
- `HybridRetentionDefaults::constrained()` / `balanced()` / `high_capacity()` all return `Self` with identical data — the distinction is only represented in `InstallProfile::default_storage_policy`; looks like the `HybridRetentionDefaults` API is not yet fully differentiated per profile.
