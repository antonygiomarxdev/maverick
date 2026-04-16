# STACK — Maverick Codebase Map
_Generated: 2026-04-16_

## Summary
Maverick is a Rust workspace (edition 2021) targeting offline-first LoRaWAN edge runtimes. All crates share a single `[workspace.package]` version (`0.1.4`) and a unified dependency graph managed through `[workspace.dependencies]`. The project builds to statically-linked Linux binaries for x86_64, aarch64, and armv7 targets, released via GitHub Actions.

---

## Language

| Language | Version | Where Used |
|----------|---------|------------|
| Rust | stable (no pinned toolchain file; CI uses `dtolnay/rust-toolchain@stable`) | All crates |

- Edition: `2021` (all crates inherit `edition.workspace = true`)
- No `rust-toolchain.toml` detected; toolchain is CI-resolved as `stable`

---

## Runtime & Package Manager

| Item | Detail |
|------|--------|
| Runtime | Native binary (no VM/interpreter) |
| Package manager | Cargo (lockfile `Cargo.lock` committed) |
| Async runtime | `tokio 1.51.1` (`features = ["full"]` in edge runtime; `["rt", "sync"]` in SQLite adapter; `["net", "time", "sync"]` in UDP adapter) |

---

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

---

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

---

## Build Configuration

### Release Profile (`Cargo.toml` `[profile.release]`)
| Setting | Value | Effect |
|---------|-------|--------|
| `lto` | `true` | Link-time optimisation (cross-crate inlining) |
| `codegen-units` | `1` | Single codegen unit (maximises optimisation, slower compile) |
| `panic` | `"abort"` | No unwinding; reduces binary size |
| `strip` | `true` | Strip debug symbols from release binary |

### Cargo Aliases (`.cargo/config.toml`)
```
cargo lint       → clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt-check  → fmt --all --check
```

### Workspace Lint Baseline (`Cargo.toml` `[workspace.lints]`)
| Lint | Level |
|------|-------|
| `rust::unused_must_use` | deny |
| `rust::unsafe_op_in_unsafe_fn` | warn |
| `clippy::dbg_macro` | deny |
| `clippy::todo` | warn |
| `clippy::unimplemented` | warn |

---

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

---

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

---

## Gaps / Unknowns

- No `rust-toolchain.toml` is present; the exact stable Rust version is CI-resolved at build time and not reproducibly pinned in the repo
- `maverick-cloud-core` crate exists with sync ingestion contracts but no cloud deployment infrastructure or binary is present
- `sysinfo 0.30.x` is at workspace level but only `maverick-runtime-edge` and `maverick-extension-tui` use it; could be scoped per-crate
- `maverick-adapter-radio-udp` description notes "Semtech-style path to be implemented" — the full GWMP downlink path is partially stubbed (`stub.rs`, `udp_downlink.rs`)
