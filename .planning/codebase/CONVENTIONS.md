# Coding Conventions

**Analysis Date:** 2026-04-16

## Naming Patterns

**Files:**
- Rust source: `snake_case.rs` (e.g., `uplink_repository.rs`, `ingest_uplink.rs`)
- Module files: `mod.rs` for module roots, peer files for submodules
- Test files: co-located in `tests/` subdirectory or inline with `#[cfg(test)]`

**Modules:**
- `snake_case` (e.g., `maverick_core::ports`, `maverick_adapter_persistence_sqlite::persistence`)

**Types (Structs, Enums, Traits, Type Aliases):**
- `PascalCase` for structs, enums, traits, and type aliases
- Acronyms kept as-is when matching protocol specs (e.g., `LoRaWAN10xClassA`, `DevAddr`, `DevEui`, `GatewayEui`)
- Domain value objects use newtype structs: `struct DevEui(Eui64)`, `struct DevAddr(u32)`

**Functions and Methods:**
- `snake_case` (e.g., `build_b0_uplink`, `compute_mic`, `send_downlink`)
- Command-handler functions use `run_<verb>_<noun>` pattern: `run_radio_ingest_once`, `run_health`, `run_probe`
- Constructor helpers use `open`, `new`, or `bind_*`: `SqlitePersistence::open`, `ResilientRadioTransport::new`

**Variables:**
- `snake_case` (e.g., `uplink_frame_counter`, `dev_addr`)
- Hex literals use underscore separators for readability: `DevAddr(0x04_03_02_01)`

**Constants:**
- `SCREAMING_SNAKE_CASE` in dedicated files (e.g., `cli_constants.rs`, `limits.rs`)
- Examples: `DEFAULT_DATA_DIR`, `DEFAULT_BACKOFF_BASE`, `EDGE_DB_FILENAME`

## Code Style

**Formatting:**
- Tool: `rustfmt` with configuration in `rustfmt.toml`
- Max line width: 100 characters
- Indentation: 4 spaces
- Newline style: Unix (`LF`)
- Field init shorthand: enabled
- Try shorthand: enabled
- Imports reordered: `reorder_imports = true`, `reorder_modules = true`

**Linting:**
- Tool: `clippy` with `-D warnings` (warnings are errors in CI)
- Workspace lint baseline in `Cargo.toml`:
  - `rust::unused_must_use = deny`
  - `rust::unsafe_op_in_unsafe_fn = warn`
  - `clippy::dbg_macro = deny`
  - `clippy::todo = warn`
  - `clippy::unimplemented = warn`
- Clippy thresholds in `.clippy.toml`:
  - Cognitive complexity: 25
  - Too-many-lines threshold: 100
  - Large error threshold: 256 bytes
  - Trivial copy size limit: 128 bytes

**CI Enforcement:**
```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
```

## Import Organization

**Order (via rustfmt):**
1. Standard library (`std`, `core`, `alloc`)
2. External crates (alphabetical)
3. Local `crate::` imports
4. Local `super::` imports

**Workspace Dependencies:**
- Accessed via `maverick_*` crate names (e.g., `maverick_core::ports`)

## Error Handling

**Central Error Type:**
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

**thiserror Derive:** `maverick-core` uses `thiserror` for `AppError`. No `anyhow`.

**Error Construction:** String messages formatted inline at construction site:
```rust
AppError::Infrastructure(format!("create data dir {}: {e}", parent.display()))
AppError::InvalidInput(format!("gwmp rxpk data base64: {e}"))
```

**Panic Policy:**
- `panic = "abort"` in release profile
- `unwrap()` permitted only in tests; `expect()` preferred over `unwrap()` with debug message
- Workspace lint `#[deny(unused_must_use)]` enforces `AppResult` is never silently dropped

## Logging

**Framework:** `tracing` (workspace dep) for structured logging; `tracing-subscriber` for initialization

**Initialization:** Only in top-level binary (`maverick-runtime-edge/src/main.rs`):
```rust
tracing_subscriber::fmt()
    .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
    .init();
```
Filter level via `RUST_LOG` env var.

**Usage Style:** Structured fields, not interpolated strings:
```rust
tracing::info!(
    snapshot_version = report.capability_snapshot.snapshot_version,
    backend_id = report.capability_snapshot.backend_id,
    "ingest capability snapshot (startup)"
);
```

**`println!` Policy:** Reserved for CLI outputs producing machine-readable JSON. Diagnostics always use `tracing`.

## Async Patterns

**Runtime:** Tokio with `features = ["full"]`. `#[tokio::main]` on binary entrypoints.

**async-trait:** Every port trait requiring async uses `#[async_trait]`:
```rust
#[async_trait]
pub trait SessionRepository: Send + Sync {
    async fn get_by_dev_addr(&self, dev_addr: DevAddr) -> AppResult<Option<SessionSnapshot>>;
    async fn upsert(&self, session: &SessionSnapshot) -> AppResult<()>;
}
```

**Blocking Work:** SQLite uses `tokio::task::spawn_blocking`:
```rust
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

**Timeout Pattern:** `tokio::time::timeout` wraps transport calls:
```rust
match tokio::time::timeout(timeout_d, async move { inner.send_downlink(&frame).await }).await {
    Ok(Ok(())) => { ... }
    Err(_elapsed) => { last_err = Some(AppError::Infrastructure(...)); }
}
```

**Mutex:** `tokio::sync::Mutex` for async-safe shared state; `std::sync::Mutex` guards SQLite connection.

## Module Design

**Layer Isolation (Hexagonal Architecture):**
- `maverick-domain`: pure value objects/entities, zero dependencies on other workspace crates
- `maverick-core`: use cases + port traits, no I/O crates (no HTTP, DB, sockets)
- `maverick-adapter-*`: implement ports, depend on `maverick-core` + `maverick-domain`
- `maverick-runtime-*`: composition roots, wire everything together

**Port Trait Organization:**
Each port trait in `crates/maverick-core/src/ports/`:
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

**Use-Case Pattern:** Each use case is a struct with `execute` method:
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

**Public API:** `lib.rs` re-exports public items with `pub use`. Minimal public surface.

## Serde Patterns

**Attribute Usage:**
```rust
#[serde(rename_all = "snake_case")]
pub enum UplinkBackendKind { GwmpUdp }

pub struct LnsConfigDocument {
    #[serde(default)]
    pub applications: Vec<ApplicationEntry>,
}
```

**Feature-Gated Serde:** Domain crate gates serde behind a feature:
```rust
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SessionSnapshot { ... }
```

## CLI Patterns

**Clap Derive:**
```rust
#[derive(Parser)]
struct Cli { ... }

#[derive(Subcommand)]
enum Commands { ... }
```

Subcommand dispatch via `match cli.command { ... }`.

## Git Conventions

**Commit Messages:** Clear, imperative style
- Example: `Add downlink retry transition in sqlite repository`
- Reference issues: `Fix session upsert (closes #123)`

**Branch Naming:**
- Feature: `feature/description`
- Bugfix: `fix/description`
- Issue-tied: `123-feature-name`

**PR Requirements:**
- [ ] `cargo check` compiles
- [ ] `cargo test --workspace` passes
- [ ] `cargo fmt --all --check` passes
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes
- [ ] Docs updated when behavior changes
- [ ] PR description includes motivation and verification steps

## Tooling Commands

**Cargo Aliases** (`.cargo/config.toml`):
```bash
cargo fmt-check    # cargo fmt --all --check
cargo lint         # cargo clippy --workspace --all-features -- -D warnings
```

**Full Verification Sequence:**
```bash
cargo check
cargo test --workspace
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

---

*Convention analysis: 2026-04-16*
