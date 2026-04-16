<!-- GSD:project-start source:PROJECT.md -->
## Project

**Maverick**

Maverick is an offline-first, local LoRaWAN Network Server (LNS) designed for edge deployments where internet connectivity is unreliable or absent. It runs on any Linux hardware — from a Raspberry Pi to an x86 server — reads LoRa radio data directly or via a packet forwarder, and persists every uplink to a local SQLite database without depending on any cloud. The architecture is a small, rock-solid core surrounded by a fully extensible layer of isolated output plugins (HTTP, MQTT, cloud sync, web UI, etc.) that the community can build without ever touching the LNS core.

**Core Value:** Never lose a LoRaWAN uplink — from radio to SQLite, data is preserved regardless of internet connectivity, extension state, or process restarts.

### Constraints

- **Tech Stack**: Rust — no runtime changes; hexagonal architecture must be maintained
- **Offline-first**: Zero cloud calls in the core runtime; all persistence is local SQLite
- **Process isolation**: Extensions are separate processes, never in-process plugins in the core
- **Hardware**: Linux only; must run on armv7 (Raspberry Pi 3) with ≤512 MB RAM
- **Resilience**: The LNS core must be supervised and self-healing; packet loss = failure
- **Compatibility**: Existing `lns-config.toml` format must remain valid; no breaking config changes in v1
<!-- GSD:project-end -->

<!-- GSD:stack-start source:codebase/STACK.md -->
## Technology Stack

## Summary
## Language
| Language | Version | Where Used |
|----------|---------|------------|
| Rust | stable (no pinned toolchain file; CI uses `dtolnay/rust-toolchain@stable`) | All crates |
- Edition: `2021` (all crates inherit `edition.workspace = true`)
- No `rust-toolchain.toml` detected; toolchain is CI-resolved as `stable`
## Runtime & Package Manager
| Item | Detail |
|------|--------|
| Runtime | Native binary (no VM/interpreter) |
| Package manager | Cargo (lockfile `Cargo.lock` committed) |
| Async runtime | `tokio 1.51.1` (`features = ["full"]` in edge runtime; `["rt", "sync"]` in SQLite adapter; `["net", "time", "sync"]` in UDP adapter) |
## Workspace Crates
| Crate | Binary Name | Role |
|-------|-------------|------|
| `maverick-domain` | — (library) | Pure domain types and value objects (no I/O); `DevAddr`, `DevEui`, `SessionSnapshot`, `RegionId`, `LoRaWANVersion`, `DeviceClass` |
| `maverick-core` | — (library) | Application kernel: use cases, ports (traits), protocol policies (`LoRaWAN10xClassA`) |
| `maverick-extension-contracts` | — (library) | Stable sync envelope contracts (v1.x forward-compatible); cloud/extension boundary |
| `maverick-extension-tui` | `maverick-edge-tui` / `maverick` | Optional terminal UX operator console; interactive menus, setup wizard, delegates to `maverick-edge` subprocess |
| `maverick-adapter-radio-udp` | — (library) | Semtech GWMP-over-UDP radio transport: packet parsing, resilient circuit-breaker wrapper, downlink sender |
| `maverick-adapter-persistence-sqlite` | — (library) | SQLite persistence adapter: sessions, uplinks, audit events, LNS config mirror |
| `maverick-runtime-edge` | `maverick-edge` | Edge runtime composition root: CLI, setup, ingest loop, health, probe, config management |
| `maverick-cloud-core` | — (library) | Cloud/hub kernel contracts; sync ingestion, no edge coupling |
| `maverick-integration-tests` | — (test-only, `publish = false`) | End-to-end integration test harness |
## Key Dependencies (Locked Versions)
### Core Async & I/O
| Crate | Locked Version | Purpose |
|-------|---------------|---------|
| `tokio` | 1.51.1 | Async runtime; all I/O (UDP sockets, timers, sync primitives) |
| `async-trait` | 0.1.x | `async fn` in traits (used for port traits in `maverick-core`) |
### Serialisation
| Crate | Locked Version | Purpose |
|-------|---------------|---------|
| `serde` | 1.0.228 | Derive-based serialisation on all domain/core types |
| `serde_json` | 1.x | JSON encode/decode for GWMP packet payloads and edge API responses |
| `toml` | 0.8.x | Parse `lns-config.toml` declarative LNS configuration file |
| `base64` | 0.22.1 | Decode base64-encoded LoRaWAN payload in GWMP `rxpk.data` field |
### Persistence
| Crate | Locked Version | Purpose |
|-------|---------------|---------|
| `rusqlite` | 0.33.0 | SQLite client; `features = ["bundled"]` — SQLite is statically compiled in, no system lib needed |
### CLI
| Crate | Locked Version | Purpose |
|-------|---------------|---------|
| `clap` | 4.6.0 | CLI parsing with derive macros; `features = ["derive", "env"]` (env var fallback for flags) |
### Observability
| Crate | Locked Version | Purpose |
|-------|---------------|---------|
| `tracing` | 0.1.44 | Structured log/trace instrumentation |
| `tracing-subscriber` | 0.3.x | Log sink; `env-filter` feature for `RUST_LOG` env var control |
### System Inspection
| Crate | Locked Version | Purpose |
|-------|---------------|---------|
| `sysinfo` | 0.30.13 | Hardware probe: total memory, CPU info; used by `run_probe` and TUI `ApplyProfile` auto-detection |
### Error Handling
| Crate | Locked Version | Purpose |
|-------|---------------|---------|
| `thiserror` | 1.x | Derive-based `Error` implementations in `maverick-domain`, `maverick-core`, `maverick-adapter-persistence-sqlite` |
### Test-Only
| Crate | Locked Version | Purpose |
|-------|---------------|---------|
| `tempfile` | 3.27.0 | Temporary SQLite files in integration tests |
## Build Configuration
### Release Profile (`Cargo.toml` `[profile.release]`)
| Setting | Value | Effect |
|---------|-------|--------|
| `lto` | `true` | Link-time optimisation (cross-crate inlining) |
| `codegen-units` | `1` | Single codegen unit (maximises optimisation, slower compile) |
| `panic` | `"abort"` | No unwinding; reduces binary size |
| `strip` | `true` | Strip debug symbols from release binary |
### Cargo Aliases (`.cargo/config.toml`)
### Workspace Lint Baseline (`Cargo.toml` `[workspace.lints]`)
| Lint | Level |
|------|-------|
| `rust::unused_must_use` | deny |
| `rust::unsafe_op_in_unsafe_fn` | warn |
| `clippy::dbg_macro` | deny |
| `clippy::todo` | warn |
| `clippy::unimplemented` | warn |
## CI/CD
### CI Pipeline (`.github/workflows/ci.yml`)
- Runs on `ubuntu-latest` for push to `main` and all PRs
- Jobs: `lint` (rustfmt + clippy `-D warnings`), `test` (`cargo test --workspace`), `audit` (`cargo-audit`)
- Uses `Swatinem/rust-cache@v2` for dependency caching
### Release Pipeline (`.github/workflows/release.yml`)
- Triggered on `v*` tags or manual `workflow_dispatch`
- Build container: `rust:1-bookworm` (Debian Bookworm baseline for glibc compatibility)
- Cross-compilation toolchains installed via `apt-get` (gcc cross compilers + sysroot headers)
- Builds two binaries per target: `maverick-edge` and `maverick-edge-tui`
- Release artifacts: `.tar.gz` archives + `.sha256` checksums uploaded to GitHub Releases via `softprops/action-gh-release@v2`
## Target Environments
| Target Triple | Arch | Typical Hardware |
|---------------|------|-----------------|
| `x86_64-unknown-linux-gnu` | x86_64 | VPS, x86 gateways |
| `aarch64-unknown-linux-gnu` | ARM64 | Raspberry Pi 4+, modern ARM gateways |
| `armv7-unknown-linux-gnueabihf` | ARMv7 | Raspberry Pi 3, older ARM gateways |
- All targets: Linux only (no Windows/macOS support)
- SQLite is bundled (no system SQLite dependency at runtime)
- No container/Docker distribution; bare binary + installer script (`scripts/install-linux.sh`)
- `MAVERICK_DATA_DIR` env var controls data directory (default: `data/`, production: `/var/lib/maverick`)
## Gaps / Unknowns
- No `rust-toolchain.toml` is present; the exact stable Rust version is CI-resolved at build time and not reproducibly pinned in the repo
- `maverick-cloud-core` crate exists with sync ingestion contracts but no cloud deployment infrastructure or binary is present
- `sysinfo 0.30.x` is at workspace level but only `maverick-runtime-edge` and `maverick-extension-tui` use it; could be scoped per-crate
- `maverick-adapter-radio-udp` description notes "Semtech-style path to be implemented" — the full GWMP downlink path is partially stubbed (`stub.rs`, `udp_downlink.rs`)
<!-- GSD:stack-end -->

<!-- GSD:conventions-start source:CONVENTIONS.md -->
## Conventions

## Summary
## Naming Conventions
### Crates
- `maverick-domain` — pure value objects / entities
- `maverick-core` — use cases + port traits (no I/O)
- `maverick-adapter-*` — port implementations (e.g., `maverick-adapter-persistence-sqlite`, `maverick-adapter-radio-udp`)
- `maverick-runtime-*` — composition roots / binaries (e.g., `maverick-runtime-edge`)
- `maverick-extension-*` — optional operator tooling (e.g., `maverick-extension-tui`)
- `maverick-integration-tests` — cross-crate integration test harness
### Files
### Types
- `AppError`, `AppResult<T>` — application-level error and result alias
- `SqlitePersistence`, `UdpDownlinkTransport`, `GwmpUplinkBatch`
- `LoRaWAN10xClassA` — acronym kept as-is when it matches the protocol spec name
- `DevAddr`, `DevEui`, `GatewayEui` — domain value objects as newtype structs
### Functions and methods
- `run_radio_ingest_once`, `run_radio_ingest_supervised`, `run_health`, `run_probe`
- `run_config_init`, `run_config_load`, `run_config_approve_device`
- `SqlitePersistence::open(path, policy, options)`
- `ResilientRadioTransport::new(inner, policy)`
- `UdpDownlinkTransport::bind_ephemeral(addr)`
### Constants
### Enum variants
## Error Handling
### Central error type
#[derive(Debug, Error)]
### `thiserror` — domain crate only
### Error construction style
### `map_err` adapter pattern
### `?` propagation
### Panic policy
## Async Patterns
### Runtime
#[tokio::main]
### `async-trait`
#[async_trait]
### Blocking work on a thread-pool
### Timeout pattern
### Mutex in async context
## Logging and Tracing
### Crate
### Initialization
### Usage style
### No `println!` on the hot path
## Module Organization
### Layer isolation rule
### Port trait pattern (hexagonal architecture)
### Use-case files
### In-module unit tests
#[cfg(test)]
## Code Style Rules
### Workspace lint policy (`Cargo.toml`)
### `#[allow(...)]` usage
### Serde attributes
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
### Feature-gated `serde` in domain crate
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
### Struct literal constants named with underscores for readability
### Clap CLI pattern
## Gaps / Unknowns
- No `rustfmt.toml` found — default `rustfmt` settings are assumed but not enforced via checked-in config.
- No CI configuration was read; unclear if `cargo clippy -- -D warnings` is enforced in CI.
- `maverick-extension-tui` (`main.rs`) has a synchronous `main() -> Result<(), String>` rather than `#[tokio::main]` — consistent with its role as a synchronous subprocess orchestrator, but worth noting as an exception.
- `HybridRetentionDefaults::constrained()` / `balanced()` / `high_capacity()` all return `Self` with identical data — the distinction is only represented in `InstallProfile::default_storage_policy`; looks like the `HybridRetentionDefaults` API is not yet fully differentiated per profile.
<!-- GSD:conventions-end -->

<!-- GSD:architecture-start source:ARCHITECTURE.md -->
## Architecture

## Summary
## Overall Pattern
- The inner hexagon contains `maverick-domain` (pure value objects) and `maverick-core` (use cases + port traits).
- The outer ring contains adapter crates (`maverick-adapter-radio-udp`, `maverick-adapter-persistence-sqlite`) that implement the port traits.
- The composition root (`maverick-runtime-edge`) is the only place where adapters are instantiated and wired to use cases. Neither `maverick-domain` nor `maverick-core` import any adapter or I/O crate.
- An optional operator console (`maverick-extension-tui`) shells out to `maverick-edge` rather than linking against adapter crates directly.
## Layers
- Purpose: Pure value objects and identifiers; zero runtime or I/O dependencies.
- Location: `crates/maverick-domain/src/`
- Contains: `DevEui`, `GatewayEui`, `DevAddr`, `Eui64`, `SessionSnapshot`, `DeviceClass`, `LoRaWANVersion`, `RegionId`.
- Depends on: nothing (serde is an optional feature gate, not a hard dep).
- Used by: `maverick-core`, both adapters, `maverick-runtime-edge`.
- Purpose: Use cases, port trait definitions, protocol capability modules, storage policy, health types.
- Location: `crates/maverick-core/src/`
- Depends on: `maverick-domain` only.
- Used by: both adapters and the runtime.
- Key sub-modules:
- `maverick-adapter-radio-udp` (`crates/maverick-adapter-radio-udp/src/`): Implements inbound GWMP/UDP parsing (no port trait impl yet — produces `UplinkObservation` structs fed to use cases by the runtime); implements `RadioTransport` for UDP downlink with `ResilientRadioTransport` circuit-breaker wrapper.
- `maverick-adapter-persistence-sqlite` (`crates/maverick-adapter-persistence-sqlite/src/`): `SqlitePersistence` implements `SessionRepository`, `UplinkRepository`, `AuditSink`, and `StoragePressureSource`. Also exposes LNS operator tables (`lns_applications`, `lns_devices`, `lns_pending`, `lns_meta`).
- Purpose: CLI entrypoint; wires adapters to use cases; owns hardware probing and install profile selection.
- Location: `crates/maverick-runtime-edge/src/`
- Binary: `maverick-edge`
- Depends on: `maverick-core`, `maverick-domain`, both adapters.
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
## Core Domain Model
- `Eui64([u8; 8])` — base EUI type.
- `DevEui(Eui64)` — device 64-bit identifier.
- `GatewayEui(Eui64)` — gateway 64-bit identifier.
- `DevAddr(u32)` — 32-bit device address (assigned after OTAA or set in ABP).
- `SessionSnapshot` — minimal session view: `dev_eui`, `dev_addr`, `region`, `class`, `uplink_frame_counter`, `downlink_frame_counter`, `application_id`.
- `DeviceClass` — `ClassA | ClassB | ClassC` (only ClassA active in v1).
- `LoRaWANVersion` — `V1_0_x` only.
- `RegionId` — `Eu868 | Us915 | Au915 | As923 | Eu433`.
- `UplinkObservation` — normalized uplink crossing the adapter→core boundary: gateway EUI, dev_addr, region, f_cnt, f_port, payload bytes, RSSI, SNR.
- `DownlinkFrame` — outbound payload to send via transport adapter.
- `ProtocolCapability` trait: `id()`, `supports(version, class, region)`, `validate_uplink(ctx) -> ProtocolDecision`.
- `ProtocolDecision` variants: `Accept`, `RejectNoSession`, `RejectRegionMismatch`, `RejectUnsupportedClass`, `RejectDuplicateFrameCounter`.
- `LoRaWAN10xClassA` — concrete implementation; stateless struct.
## Data Flow
## Resilience Pattern
- Per-attempt timeout (`tokio::time::timeout`).
- Exponential backoff retry loop.
- Three-state circuit breaker: `Closed → Open → HalfOpen → Closed`.
- All state held in `AtomicU32` + `Mutex<CircuitState>` (no external dependency).
## Extension / Cloud Sync Surface
## Error Handling Strategy
- `Domain` — business rule violations (bad FCnt, unknown DevAddr).
- `NotFound` — missing entity.
- `InvalidInput` — parse/validation failures.
- `Infrastructure` — SQLite, I/O, task join errors.
- `CircuitOpen` — radio transport circuit open.
## Crate Dependency Graph
```
```
## Gaps / Unknowns
- `DeviceRepository` and `DownlinkRepository`/`DownlinkEnqueue` port traits are defined but have no adapter implementations; downlink scheduling is not yet wired end-to-end.
- OTAA join handling (JoinRequest/JoinAccept exchange) is absent; only ABP and pre-configured OTAA sessions ingested via `config load` are operational.
- `HybridRetentionDefaults` `constrained()`/`balanced()`/`high_capacity()` constructors all return the same struct; the profile differentiation only manifests in `InstallProfile::default_storage_policy()` — the named constructors are vestigial.
- Cloud sync (`SyncBatchEnvelopeV1` / `HubSyncIngest`) is defined but has no runtime trigger or scheduler.
- Region inference in `gwmp.rs::infer_region()` uses overlapping frequency ranges that could produce incorrect `RegionId` for borderline frequencies.
<!-- GSD:architecture-end -->

<!-- GSD:skills-start source:skills/ -->
## Project Skills

| Skill | Description | Path |
|-------|-------------|------|
| rust-best-practices | >- Idiomatic Rust and engineering hygiene: Clippy, Option/Result patterns, ownership in APIs, iterators, unsafe discipline, async Send boundaries, and public API docs. Use when writing or reviewing Rust, onboarding idioms, or when the user asks for Rust best practices, idioms, or lint hygiene. | `.cursor/skills/rust-best-practices/SKILL.md` |
| rust-clean-code | >- Clean-code guidance for Rust: function size, modules, error handling, naming, and tests. Use during refactors, readability reviews, or when the user asks for cleaner or more maintainable code without a specific architecture topic. | `.cursor/skills/rust-clean-code/SKILL.md` |
| rust-linter-configuration | >- Documents Maverick Rust lint and format setup: rustfmt.toml, Clippy flags in CI, cargo aliases, and policy for allow/expect attributes. Use when configuring editors, fixing CI lint failures, adding clippy.toml or rustfmt options, or when the user mentions linters, rustfmt, Clippy, or -D warnings. | `.cursor/skills/rust-linter-configuration/SKILL.md` |
| rust-no-magic-values | >- Applies common production Rust practices to remove magic literals: named consts, enums, newtypes, and centralized schema strings. Use when writing or reviewing Rust, when the user mentions magic strings/numbers, duplicated literals, or when designing core versus adapter layers. | `.cursor/skills/rust-no-magic-values/SKILL.md` |
| rust-solid-hexagonal | >- Applies SOLID and hexagonal architecture in Rust: traits as ports, implementations in adapters, dependency inversion. Use when designing maverick-core/domain versus adapters, adding persistence or transport, or when the user mentions SOLID, ports, adapters, or coupling. | `.cursor/skills/rust-solid-hexagonal/SKILL.md` |
<!-- GSD:skills-end -->

<!-- GSD:workflow-start source:GSD defaults -->
## GSD Workflow Enforcement

Before using Edit, Write, or other file-changing tools, start work through a GSD command so planning artifacts and execution context stay in sync.

Use these entry points:
- `/gsd-quick` for small fixes, doc updates, and ad-hoc tasks
- `/gsd-debug` for investigation and bug fixing
- `/gsd-execute-phase` for planned phase work

Do not make direct repo edits outside a GSD workflow unless the user explicitly asks to bypass it.
<!-- GSD:workflow-end -->



<!-- GSD:profile-start -->
## Developer Profile

> Profile not yet configured. Run `/gsd-profile-user` to generate your developer profile.
> This section is managed by `generate-claude-profile` -- do not edit manually.
<!-- GSD:profile-end -->
