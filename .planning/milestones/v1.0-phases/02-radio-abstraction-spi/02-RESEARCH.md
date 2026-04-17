# Phase 2: Radio Abstraction & SPI - Research

**Researched:** 2026-04-16
**Domain:** Rust C FFI (libloragw), Cargo feature gates, async port traits, cross-compilation
**Confidence:** MEDIUM — `loragw-hal` as a published crates.io package was NOT found; the implementation strategy shifts to an in-tree `-sys` crate pattern proven by ChirpStack Concentratord

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** New trait `UplinkSource` in `maverick-core::ports`, using `async-trait` consistent with all other port traits.
- **D-02:** API shape: `async fn next_batch(&self) -> AppResult<Vec<UplinkObservation>>` — one blocking-style call that returns a batch (empty vec on timeout/idle is OK).
- **D-03:** Stream-based API rejected — more complex, harder to test, harder to wrap blocking SPI HAL calls.
- **D-04:** `GwmpUdpUplinkSource` in `maverick-adapter-radio-udp` wraps existing UDP recv + `parse_push_data` logic, implementing `UplinkSource`.
- **D-05:** The ingest loop in `gwmp_loop.rs` is refactored to call `source.next_batch()` instead of raw socket operations.
- **D-06:** Use `loragw-hal` crate (C FFI bindings to Semtech's libloragw) — proven real-hardware compatibility with SX1302/SX1303, supports RAK Pi HAT.
- **D-07:** Pure-Rust SPI/register implementation rejected — out of scope for v1.
- **D-08:** New crate `maverick-adapter-radio-spi` wraps `loragw-hal`, implements `UplinkSource`.
- **D-09:** The `maverick-adapter-radio-spi` crate is gated behind a `spi` feature flag in `maverick-runtime-edge`.
- **D-10:** CI cross-compile for armv7/aarch64 must install libloragw sysroot headers when `spi` feature is active.
- **D-11:** New `[radio]` section in `lns-config.toml` with `backend = "spi"/"udp"` and optional `spi_path`.
- **D-12:** Absent `[radio]` section defaults to `"udp"` — all existing configs remain valid.
- **D-13:** `LnsConfigDocument` gains `radio: Option<RadioConfig>` with `#[serde(default)]`.
- **D-14:** Hardware registry ships as `hardware-registry.toml` bundled in release archive, NOT compiled in.
- **D-15–17:** RAK Pi HAT (RAK2287/RAK5146) is initial verified entry; registry is read-only documentation.

### Claude's Discretion
- Exact `loragw-hal` version and initialization sequence
- Exact `next_batch` idle timeout value for UDP adapter
- File path for `hardware-registry.toml` in release layout

### Deferred Ideas (OUT OF SCOPE)
- SPI downlink (TX) — Phase 3
- Automatic hardware detection (board auto-probe to select spi_path) — Phase 5
- USB concentrator adapters — community extension, out of v1 scope
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| RELI-05 | `UplinkSource` port trait abstracts radio backend so ingest loop is radio-agnostic | Defines the new async-trait in `maverick-core::ports`; both UDP and SPI adapters implement it; ingest loop only calls `source.next_batch()` |
| RADIO-01 | Maverick reads LoRa frames directly from SX1302/SX1303 via SPI on Raspberry Pi without external packet forwarder | Implemented by `maverick-adapter-radio-spi` wrapping libloragw C HAL via an in-tree `-sys` crate |
| RADIO-02 | SPI radio adapter implements the `UplinkSource` port trait alongside the existing UDP adapter | `SpiUplinkSource` in `maverick-adapter-radio-spi` implements `UplinkSource`; blocking HAL calls wrapped in `spawn_blocking` |
| RADIO-03 | Radio backend selectable via config (SPI or UDP) — UDP remains for dev/testing/simulator use | New optional `[radio]` TOML section; `#[serde(default)]` preserves backward compat |
| RADIO-04 | Hardware compatibility registry documents RAK Pi as verified-supported hardware | `hardware-registry.toml` file in release archive; community-extensible TOML format |
| CORE-04 | Hardware compatibility registry (TOML) ships with verified/untested/unsupported classification — community can contribute without code changes | Aligns exactly with D-14–17; registry has a `status` field per entry |
</phase_requirements>

---

## Summary

Phase 2 adds the `UplinkSource` port trait so the ingest loop becomes radio-backend-agnostic (RELI-05), then implements a concrete SPI adapter backed by Semtech's `libloragw` C library (RADIO-01/02). The key discovery from this research is that **`loragw-hal` is not a published crates.io package** — the name appears to have been used in the CONTEXT.md as a conceptual reference. The proven strategy used by production Rust LoRa infrastructure (ChirpStack Concentratord) is an **in-tree `-sys` crate pattern**: vendor the `libloragw` C sources inside the new `maverick-adapter-radio-spi` crate, compile them via `cc` in `build.rs` when the `spi` Cargo feature is active, and expose thin safe Rust wrappers over the generated FFI.

The existing cross-compilation infrastructure in `release.yml` already installs ARM sysroot headers (`libc6-dev-arm64-cross`, `libc6-dev-armhf-cross`) for the bundled SQLite build. Extending that with `libm` linkage is trivial since libm is part of glibc — no additional apt packages are needed. The `spi` feature ensures CI lint/test jobs on x86 never attempt to compile the C sources, keeping developer ergonomics intact.

The `UplinkSource` async trait pattern matches the existing `SessionRepository` / `AuditSink` pattern exactly: declare with `#[async_trait]` in `maverick-core::ports`, implement with `spawn_blocking` for blocking I/O. The ingest loop refactor is surgical: replace the raw UDP recv loop body with a `source.next_batch().await?` call.

**Primary recommendation:** Create an in-tree `libloragw-sys` submodule inside `maverick-adapter-radio-spi` that vendors the libloragw C sources and compiles them via `cc` behind `CARGO_FEATURE_SPI`. The safe Rust wrapper translates `lgw_pkt_rx_s` structs into `UplinkObservation` values and calls `spawn_blocking` for all HAL calls.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| `UplinkSource` trait definition | Core (port trait) | — | Hexagonal arch: traits in core, impls in adapters |
| UDP ingest source impl | Adapter (radio-udp) | — | Wraps existing parse_push_data + UDP recv |
| SPI ingest source impl | Adapter (radio-spi) | — | Wraps libloragw C HAL, translates to UplinkObservation |
| libloragw C compilation | Adapter (radio-spi build.rs) | — | cc build-dependency, feature-gated |
| Radio backend selection | Runtime composition root | Config (lns_config.rs) | Runtime reads config, instantiates correct source |
| Hardware registry | Static file (release archive) | — | Runtime does not parse; operator reference only |
| Config backward-compat | Core (lns_config.rs) | — | #[serde(default)] on Option<RadioConfig> |
| spawn_blocking wrapper | Adapter (radio-spi) | — | Keeps async contract; mirrors SQLite adapter pattern |

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `async-trait` | 0.1 (workspace) | `async fn` in `UplinkSource` trait | Already in workspace; used by all port traits |
| `cc` | 1.x | Compile libloragw C sources in build.rs | Cargo standard for C FFI compilation |
| `tokio` | 1.51 (workspace) | `spawn_blocking` for SPI HAL calls | Already in workspace; same pattern as SQLite adapter |
| `thiserror` | 1.x (workspace) | Error types in spi adapter | Already in workspace |
| `tracing` | 0.1 (workspace) | Log SPI init, receive events | Already in workspace |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `bindgen` | 0.69.x | Generate Rust FFI bindings from loragw_hal.h | Build-time only; optional build-dependency; only when regenerating bindings |
| `serde` | 1.x (workspace) | `RadioConfig` TOML deserialization | Already in workspace |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| In-tree libloragw C compilation | Published `loragw-hal` crate | `loragw-hal` does NOT exist on crates.io; in-tree is the only proven approach |
| In-tree C compilation via `cc` | `cross-rs` Docker cross-compile | `cross-rs` adds Docker dependency; existing apt-based CI already works for SQLite |
| Pre-generated `bindings.rs` checked in | `bindgen` at build time | Pre-generated bindings avoid requiring LLVM on all build machines — recommended for CI stability |

**Installation (new crate `maverick-adapter-radio-spi`):**
```toml
# crates/maverick-adapter-radio-spi/Cargo.toml
[build-dependencies]
cc = { version = "1", optional = true }

[features]
default = []
spi = ["dep:cc"]
```

```toml
# crates/maverick-runtime-edge/Cargo.toml — add:
maverick-adapter-radio-spi = { path = "../maverick-adapter-radio-spi", optional = true }

[features]
spi = ["dep:maverick-adapter-radio-spi", "maverick-adapter-radio-spi/spi"]
```

**Version verification:** `cc` is a build-time crate; current version is 1.2.x on crates.io. [VERIFIED: crates.io search results]

---

## Architecture Patterns

### System Architecture Diagram

```
lns-config.toml
    [radio] backend = "spi" / "udp"
         |
         v
maverick-runtime-edge (composition root)
    reads RadioConfig -> selects UplinkSource impl
         |
    +----+----+
    |         |
    v         v
GwmpUdpUplinkSource   SpiUplinkSource
(maverick-adapter-    (maverick-adapter-
 radio-udp)            radio-spi)
    |                     |
    | async next_batch()  | spawn_blocking -> lgw_receive()
    |                     |
    +----+----+-----------+
         |
         v (Vec<UplinkObservation>)
    ingest loop (gwmp_loop.rs)
         |
         v
    IngestUplink use case (maverick-core)
         |
         v
    SqlitePersistence (maverick-adapter-persistence-sqlite)
```

```
maverick-adapter-radio-spi/
    build.rs               # compiles libloragw C sources when CARGO_FEATURE_SPI set
    src/
        lib.rs             # pub use SpiUplinkSource
        spi_source.rs      # SpiUplinkSource: UplinkSource impl
        lgw_init.rs        # lgw_board_conf_s setup, lgw_start() / lgw_stop()
        lgw_convert.rs     # lgw_pkt_rx_s -> UplinkObservation translation
    libloragw/             # vendored C sources from lora-net/sx1302_hal
        src/               # loragw_hal.c, loragw_spi.c, etc.
        inc/               # loragw_hal.h, loragw_reg.h, etc.
    src/
        bindings.rs        # pre-generated FFI (checked in; not regenerated in CI)
```

### Pattern 1: UplinkSource Async Trait

**What:** New port trait in `maverick-core::ports` following the exact same async-trait pattern as `SessionRepository`.

**When to use:** Any radio adapter that produces `Vec<UplinkObservation>` batches.

```rust
// Source: existing maverick-core::ports::session_repository.rs pattern
// crates/maverick-core/src/ports/uplink_source.rs
use async_trait::async_trait;
use crate::error::AppResult;
use crate::ports::UplinkObservation;

#[async_trait]
pub trait UplinkSource: Send + Sync {
    /// Block until at least one uplink is available, or idle timeout elapses.
    /// Returns empty vec on timeout — never blocks indefinitely.
    async fn next_batch(&self) -> AppResult<Vec<UplinkObservation>>;
}
```

### Pattern 2: Feature-Gated C FFI Compilation in build.rs

**What:** Compile libloragw C sources only when `spi` feature is active; skip entirely on x86 dev boxes.

**When to use:** Any `build.rs` that must compile C code only for a specific optional feature.

```rust
// Source: [CITED: doc.rust-lang.org/cargo/reference/build-script-examples.html]
// crates/maverick-adapter-radio-spi/build.rs
fn main() {
    // CARGO_FEATURE_SPI is set by Cargo when the "spi" feature is active.
    if std::env::var("CARGO_FEATURE_SPI").is_err() {
        return; // x86 / CI lint job: skip C compilation entirely
    }

    let mut build = cc::Build::new();
    // List all libloragw source files
    let sources = [
        "libloragw/src/loragw_hal.c",
        "libloragw/src/loragw_reg.c",
        "libloragw/src/loragw_com.c",
        "libloragw/src/loragw_spi.c",
        "libloragw/src/loragw_sx1302.c",
        "libloragw/src/loragw_sx1302_rx.c",
        "libloragw/src/loragw_sx1302_timestamp.c",
        "libloragw/src/loragw_sx125x.c",
        "libloragw/src/loragw_sx1250.c",
        "libloragw/src/loragw_aux.c",
    ];
    for s in &sources {
        build.file(s);
    }
    build
        .include("libloragw/inc")
        .flag("-Wno-unused-parameter")
        .flag("-Wno-sign-compare")
        .compile("loragw");

    // libloragw links against libm and librt
    println!("cargo:rustc-link-lib=m");
    println!("cargo:rustc-link-lib=rt");
}
```

**CRITICAL:** `cfg!(feature = "spi")` does NOT work in build.rs. Use `std::env::var("CARGO_FEATURE_SPI")` instead. [CITED: doc.rust-lang.org/cargo/reference/build-script-examples.html]

### Pattern 3: spawn_blocking for Blocking HAL Calls

**What:** Wrap blocking libloragw C calls in `tokio::task::spawn_blocking` so the async runtime is not blocked. Identical to `SqlitePersistence::run_blocking`.

**When to use:** Any async method that calls into blocking C FFI.

```rust
// Source: crates/maverick-adapter-persistence-sqlite/src/persistence/mod.rs (existing pattern)
// crates/maverick-adapter-radio-spi/src/spi_source.rs

use std::sync::Arc;
use async_trait::async_trait;
use maverick_core::error::{AppError, AppResult};
use maverick_core::ports::{UplinkObservation, UplinkSource};

#[derive(Clone)]
pub struct SpiUplinkSource {
    inner: Arc<SpiInner>,
}

struct SpiInner {
    // The HAL is NOT Send; all HAL calls must happen on the spawn_blocking thread
    // Use a Mutex<()> as a serialization guard (same pattern as SQLite Mutex<Connection>)
    hal_lock: std::sync::Mutex<()>,
}

#[async_trait]
impl UplinkSource for SpiUplinkSource {
    async fn next_batch(&self) -> AppResult<Vec<UplinkObservation>> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || {
            let _guard = this.inner.hal_lock.lock().map_err(|_| {
                AppError::Infrastructure("spi hal mutex poisoned".to_string())
            })?;
            // SAFETY: libloragw is single-threaded; Mutex<()> enforces that
            unsafe { lgw_receive_batch() }
        })
        .await
        .map_err(|e| AppError::Infrastructure(format!("spi spawn_blocking join: {e}")))?
    }
}
```

### Pattern 4: #[serde(default)] Optional Config Section

**What:** Add `radio: Option<RadioConfig>` to `LnsConfigDocument` with `#[serde(default)]` so files without a `[radio]` section deserialize to `None` (which the runtime treats as `backend = "udp"`).

**When to use:** Any backward-compatible extension to a TOML config struct.

```rust
// Source: crates/maverick-core/src/lns_config.rs (existing patterns throughout)
// crates/maverick-core/src/lns_config.rs — addition to LnsConfigDocument

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LnsConfigDocument {
    pub schema_version: u32,
    #[serde(default)]
    pub autoprovision: AutoprovisionPolicy,
    #[serde(default)]
    pub applications: Vec<ApplicationEntry>,
    #[serde(default)]
    pub devices: Vec<DeviceEntry>,
    #[serde(default)]          // <- absent = None = udp backend
    pub radio: Option<RadioConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RadioConfig {
    #[serde(default = "default_radio_backend")]
    pub backend: RadioBackend,
    #[serde(default)]
    pub spi_path: Option<String>,     // required when backend = spi; validated at runtime
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum RadioBackend {
    #[default]
    Udp,
    Spi,
}

fn default_radio_backend() -> RadioBackend { RadioBackend::Udp }
```

### Pattern 5: GwmpUdpUplinkSource Wrapping Existing Logic

**What:** Refactor the UDP adapter so it implements `UplinkSource` by wrapping the existing `parse_push_data` call.

```rust
// Source: crates/maverick-adapter-radio-udp/src/gwmp.rs (existing parse_push_data)
// crates/maverick-adapter-radio-udp/src/uplink_source.rs (new file)

use std::time::Duration;
use async_trait::async_trait;
use tokio::net::UdpSocket;
use maverick_core::ports::{UplinkObservation, UplinkSource};
use maverick_core::error::AppResult;

pub struct GwmpUdpUplinkSource {
    socket: UdpSocket,
    idle_timeout: Duration,  // discretion: suggest 200ms
}

#[async_trait]
impl UplinkSource for GwmpUdpUplinkSource {
    async fn next_batch(&self) -> AppResult<Vec<UplinkObservation>> {
        let mut buf = vec![0u8; 4096];
        match tokio::time::timeout(self.idle_timeout, self.socket.recv_from(&mut buf)).await {
            Err(_) => Ok(vec![]),   // idle timeout — normal in supervised mode
            Ok(Err(e)) => Err(maverick_core::error::AppError::Infrastructure(
                format!("udp recv: {e}")
            )),
            Ok(Ok((n, _addr))) => {
                let batch = crate::parse_push_data(&buf[..n])?;
                Ok(batch.observations)
            }
        }
    }
}
```

### Anti-Patterns to Avoid

- **Calling HAL from async context without spawn_blocking:** `lgw_receive` is a blocking C call. Calling it directly in `async fn` will block the Tokio executor thread and starve other tasks. Always use `spawn_blocking`.
- **Using `cfg!(feature = "spi")` in build.rs:** This evaluates the HOST's features, not the crate being built. Use `std::env::var("CARGO_FEATURE_SPI")` in build.rs.
- **Linking libloragw as a system library:** libloragw has no `pkg-config` support on Raspbian and is not in apt. Always compile from vendored source via `cc`.
- **Global HAL state without a Mutex guard:** libloragw uses global C state (SPI file descriptor, internal buffers). Only one thread should call HAL functions at a time. The `Mutex<()>` in `SpiInner` enforces this.
- **Not calling lgw_stop() on Drop:** Leaving the SPI device open causes the next process start to fail on `lgw_start()`. Implement `Drop for SpiUplinkSource` to call `lgw_stop()`.
- **Panic inside spawn_blocking holding the HAL Mutex:** A panic drops the lock guard, but the Mutex becomes poisoned. Handle `lock().map_err(...)` — the same rule that motivated RELI-01.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| SX1302 register protocol | Custom SPI register driver | libloragw vendored C (via `cc`) | SX1302 has 3000+ registers, firmware loading, timestamping — decades of Semtech engineering |
| C FFI binding generation | Manual `extern "C"` declarations | Pre-generated `bindings.rs` (from bindgen once) | Manual declarations diverge from header; binding errors are runtime panics |
| C compilation at x86 | Attempting to compile ARM-only C on x86 | Feature gate the C compilation | libloragw uses Linux SPI ioctls not available on macOS/Windows; build isolation is mandatory |

**Key insight:** libloragw exists because the SX1302 initialization sequence requires loading proprietary firmware over SPI and calibrating RF chains — writing this from scratch would dwarf the rest of the Maverick codebase.

---

## Runtime State Inventory

> Phase 2 is a greenfield feature addition + refactor. No rename/migration involved.

This section is omitted — not applicable.

---

## Common Pitfalls

### Pitfall 1: loragw-hal Does Not Exist on crates.io

**What goes wrong:** Attempting to add `loragw-hal = "..."` to Cargo.toml results in "no matches found" or resolves to a completely unrelated crate.

**Why it happens:** The name was used as a conceptual reference in the discuss phase; no published crate with that exact name exists (verified 2026-04-16 against crates.io and lib.rs). The Helium project's `libloragw-sx1302-sys` exists on GitHub but is not published to crates.io.

**How to avoid:** Use the in-tree vendored C pattern (vendor libloragw sources inside the new crate, compile via `cc`). This is the exact approach used by ChirpStack Concentratord.

**Warning signs:** `cargo update` fails with "package ... not found in registry".

### Pitfall 2: Bindgen Padding Mismatch on ARM Cross-Compile

**What goes wrong:** When `bindgen` regenerates bindings during cross-compilation, struct layout may include `__bindgen_padding_0` fields that break the existing checked-in `bindings.rs`.

**Why it happens:** bindgen uses the host architecture's layout to generate offsets; cross-compiled targets may differ for structs with `f32`/alignment constraints.

**How to avoid:** Check in the `bindings.rs` generated on the target architecture (ARM). Do NOT run bindgen during CI cross-compile. Set `CARGO_FEATURE_SPI` to skip bindgen; only the pre-generated file is used. [CITED: github.com/rust-lang/rust-bindgen/issues/1709]

**Warning signs:** CI cross-compile fails with "struct has no field named `__bindgen_padding_0`" or similar.

### Pitfall 3: libloragw Global State — lgw_start/lgw_stop Symmetry

**What goes wrong:** If `lgw_start()` is called twice (e.g., process restart without clean shutdown), the second call fails because the SPI device file is already open in a previous invocation's state. On Linux, `/dev/spidev0.0` becomes locked.

**Why it happens:** libloragw stores global state in module-level C variables. There is no instance concept.

**How to avoid:** Implement `Drop for SpiUplinkSource` to call `unsafe { lgw_stop() }`. Ensure the drop runs before process exit — this aligns with the RELI-02 pattern (clean shutdown before exit).

**Warning signs:** `lgw_start()` returns `LGW_HAL_ERROR` on second process start; `/dev/spidev0.0` EBUSY errors.

### Pitfall 4: ingest loop refactor breaks UDP path

**What goes wrong:** After refactoring `gwmp_loop.rs` to call `source.next_batch()`, the UDP integration tests fail because the socket binding or timeout behavior changed.

**Why it happens:** The old loop called `socket.recv_from` directly; the new `GwmpUdpUplinkSource` owns the socket and may have different bind timing or error behavior.

**How to avoid:** Keep `run_radio_ingest_once` and `run_radio_ingest_supervised` as entry points but replace their body with `source.next_batch()` calls. Ensure existing integration tests in `operator_local_gateway_e2e.rs` pass unchanged after refactor.

**Warning signs:** `cargo test --workspace` fails in `maverick-integration-tests` with UDP recv timeout or bind errors.

### Pitfall 5: CI Lint Job Fails Because spi Feature is Not Excluded

**What goes wrong:** `cargo clippy --all-targets --all-features` (which runs in CI lint job) attempts to compile the `spi` feature, causing the C compilation to fail on x86 (missing SPI headers or cross-compiler).

**Why it happens:** `--all-features` activates all features including `spi`, triggering `build.rs` C compilation.

**How to avoid:** Change the CI lint job from `--all-features` to `--features=""` or explicitly exclude the spi feature: `cargo clippy --all-targets`. Document that `spi` must only be compiled for ARM release targets.

**Warning signs:** CI lint job fails with "cc: error: unrecognized command-line option" or missing SPI ioctl headers.

### Pitfall 6: UplinkObservation Construction from lgw_pkt_rx_s

**What goes wrong:** `lgw_pkt_rx_s` carries a raw PHY payload but does NOT split `phy_without_mic` and `wire_mic` — that split must be done in the Rust conversion layer.

**Why it happens:** `lgw_pkt_rx_s.payload` is the raw LoRaWAN PHY payload bytes including the 4-byte MIC at the end. `UplinkObservation` requires `wire_mic: [u8; 4]` and `phy_without_mic: Vec<u8>` as separate fields (added in Phase 1).

**How to avoid:** In `lgw_convert.rs`, implement the split: `wire_mic = payload[size-4..size]`, `phy_without_mic = payload[..size-4]`. Add unit tests for this conversion.

**Warning signs:** MIC verification fails for all SPI-sourced uplinks (all MICs compute as invalid).

---

## Code Examples

Verified patterns from official sources and existing codebase:

### Existing spawn_blocking Pattern (SQLite Adapter)
```rust
// Source: crates/maverick-adapter-persistence-sqlite/src/persistence/mod.rs (VERIFIED: read this file)
async fn run_blocking<T: Send + 'static>(
    &self,
    f: impl FnOnce(&SqlitePersistence) -> AppResult<T> + Send + 'static,
) -> AppResult<T> {
    let this = self.clone();
    tokio::task::spawn_blocking(move || f(&this))
        .await
        .map_err(|e| AppError::Infrastructure(format!("join blocking task: {e}")))?
}
```

### CARGO_FEATURE_* Pattern in build.rs
```rust
// Source: [CITED: doc.rust-lang.org/cargo/reference/build-script-examples.html]
fn main() {
    if std::env::var("CARGO_FEATURE_SPI").is_err() {
        return; // feature not active; skip C compilation
    }
    cc::Build::new()
        .file("libloragw/src/loragw_hal.c")
        // ... other files
        .include("libloragw/inc")
        .compile("loragw");
    println!("cargo:rustc-link-lib=m");
    println!("cargo:rustc-link-lib=rt");
}
```

### Optional dep: Feature in Cargo.toml (resolver = "2")
```toml
# Source: [CITED: doc.rust-lang.org/cargo/reference/features.html]
[dependencies]
maverick-adapter-radio-spi = { path = "../maverick-adapter-radio-spi", optional = true }

[features]
spi = ["dep:maverick-adapter-radio-spi", "maverick-adapter-radio-spi/spi"]
```

### libloragw C HAL Key Functions
```c
// Source: [CITED: github.com/lora-net/sx1302_hal/blob/master/libloragw/src/loragw_hal.c]
// Initialize and start the concentrator (blocking; takes ~2s)
int lgw_start(void);

// Shut down concentrator and close SPI device
int lgw_stop(void);

// Non-blocking poll: fetch up to max_pkt packets from SX1302 FIFO
// Returns: number of packets fetched (0..max_pkt), or LGW_HAL_ERROR
int lgw_receive(uint8_t max_pkt, struct lgw_pkt_rx_s *pkt_data);

// Key fields of lgw_pkt_rx_s:
//   freq_hz: u32       — receive frequency in Hz
//   rf_chain: u8       — 0 or 1
//   modulation: u8     — LORA=0x10
//   datarate: u32      — spreading factor (7..12 for LoRa)
//   rssic: f32         — channel RSSI in dBm
//   snr: f32           — packet SNR in dB
//   size: u16          — payload length in bytes
//   payload: [u8; 256] — raw LoRaWAN PHY frame bytes
```

### hardware-registry.toml Format
```toml
# File: hardware-registry.toml (bundled in release archive, not compiled in)
# Status values: "verified" | "untested" | "unsupported"

[[boards]]
board_name   = "RAK2287"
arch         = ["armv7", "aarch64"]
spi_device   = "/dev/spidev0.0"
concentrator = "sx1302"
status       = "verified"
notes        = "RAK Pi HAT — tested on Raspberry Pi 3/4 with RAKwireless RAK2287/RAK5146"

[[boards]]
board_name   = "Generic SX1302"
arch         = ["armv7", "aarch64"]
spi_device   = "/dev/spidev0.0"
concentrator = "sx1302"
status       = "untested"
notes        = "Community contribution — not verified by Maverick team"
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| GWMP packet forwarder as mandatory intermediary | Direct SPI via libloragw | This phase | Eliminates external process dependency; works offline without Semtech reference software |
| Hardcoded UDP recv in ingest loop | `UplinkSource` port trait | This phase | Ingest loop becomes backend-agnostic; UDP and SPI are interchangeable |
| `loragw-hal` (assumed published crate) | In-tree vendored C + cc build | Research finding | No published crate exists; vendoring is the proven production pattern |

**Deprecated/outdated:**
- Direct socket calls in `gwmp_loop.rs`: replaced by `UplinkSource::next_batch()`.
- `GwmpUdpIngressBackend` as the only radio backend identity: `UplinkBackendKind::Spi` variant added alongside.

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | libloragw requires `libm` and `librt` linkage | Standard Stack / Code Examples | Missing link flag causes undefined symbol at link time; low risk (libm/librt are standard glibc) |
| A2 | The `cc` crate can cross-compile libloragw C sources using the existing ARM sysroot env vars already set in release.yml | CI Cross-Compile Strategy | If libloragw uses Linux-specific headers not in the glibc sysroot, additional apt packages may be needed; MEDIUM risk |
| A3 | lgw_start/lgw_stop are safe to call from a single background thread protected by a Mutex<()> | spawn_blocking pattern | If libloragw uses signals or assumes main thread context, this could fail; LOW risk (proven by ChirpStack use) |
| A4 | `idle_timeout = 200ms` for UDP `next_batch()` is a reasonable default | GwmpUdpUplinkSource | If too short, CPU waste; if too long, shutdown latency; can be tuned post-implementation |
| A5 | Pre-generated bindings.rs checked in avoids bindgen padding mismatch on ARM cross-compile | Pitfall 2 | If the checked-in bindings.rs was generated on x86, struct offsets may be wrong for ARM; MEDIUM risk — bindings should be generated on ARM or using `--target` flag |

---

## Open Questions

1. **libloragw board configuration for SX1302 HAT**
   - What we know: `lgw_start()` requires a prior `lgw_board_setconf()` call with `lgw_board_conf_s` (specifying SPI device path, clock source, full duplex setting for SX1302).
   - What's unclear: Which `lgw_board_conf_s` fields must be set for RAK2287/RAK5146; some HAT-specific fields may also require `lgw_rxrf_setconf()` for each RF chain.
   - Recommendation: Reference the RAK2287 packet forwarder `global_conf.json` (public in RAK GitHub repo) for field values; hardcode sensible defaults for common RAK HAT configurations in the initial implementation.

2. **CI lint exclusion of spi feature**
   - What we know: Current CI runs `cargo clippy --all-targets --all-features -- -D warnings`.
   - What's unclear: Whether `--all-features` will activate `spi` on the x86 lint runner.
   - Recommendation: Change CI lint to `cargo clippy --all-targets -- -D warnings` (no `--all-features`) so `spi` is never activated on x86. This is a one-line CI change.

3. **Bindings.rs generation target**
   - What we know: bindgen padding issues occur when generating on x86 for ARM targets.
   - What's unclear: Whether the `lgw_pkt_rx_s` struct has alignment-sensitive fields that differ between x86 and ARM.
   - Recommendation: Generate bindings on an ARM target (or use `bindgen --target aarch64-unknown-linux-gnu`) once and check in the result. Document the regeneration procedure in the crate README.

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust stable toolchain | All compilation | ✓ | CI-resolved | — |
| `cc` crate (apt: build-essential) | C compilation in build.rs | ✓ | In `rust:1-bookworm` Docker image | — |
| `gcc-arm-linux-gnueabihf` | armv7 cross-compile with spi | ✓ | Installed in release.yml already | — |
| `gcc-aarch64-linux-gnu` | aarch64 cross-compile with spi | ✓ | Installed in release.yml already | — |
| `libm` (math) | libloragw link dep | ✓ | Part of glibc on all targets | — |
| `librt` (realtime) | libloragw timing functions | ✓ | Part of glibc on Linux; may be implicit on modern glibc | — |
| `/dev/spidev0.0` | SpiUplinkSource on hardware | ✗ (x86 CI) | — | Feature-gated: never runs on x86 |
| SX1302 HAT hardware | RADIO-01 acceptance test | ✗ (CI) | — | Integration test is manual/hardware-only |

**Missing dependencies with no fallback:**
- None — all CI dependencies are available; hardware test is explicitly out of CI scope.

**Missing dependencies with fallback:**
- `/dev/spidev0.0`: The `spi` feature ensures this is never required on x86. Manual hardware testing on RPi is the acceptance criterion.

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | cargo test (native) |
| Config file | none (workspace `cargo test --workspace`) |
| Quick run command | `cargo test --workspace -- --test-thread=1` |
| Full suite command | `cargo test --workspace` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| RELI-05 | `UplinkSource` trait: UDP impl returns observations from parse_push_data | unit | `cargo test -p maverick-adapter-radio-udp` | ❌ Wave 0 |
| RELI-05 | Ingest loop calls `source.next_batch()` and processes results identically for both backends | integration | `cargo test -p maverick-integration-tests --test operator_local_gateway_e2e` | ✅ (existing, must remain green) |
| RADIO-01 | SpiUplinkSource wraps lgw_receive and constructs correct UplinkObservation | unit (mock lgw) | `cargo test -p maverick-adapter-radio-spi --features spi` | ❌ Wave 0 |
| RADIO-02 | Both GwmpUdpUplinkSource and SpiUplinkSource satisfy UplinkSource trait at compile time | compilation | `cargo build --features spi` (ARM target only) | — |
| RADIO-03 | LnsConfigDocument with absent [radio] section deserializes to udp backend | unit | `cargo test -p maverick-core` | ❌ Wave 0 |
| RADIO-03 | LnsConfigDocument with `[radio] backend = "spi"` deserializes to spi backend | unit | `cargo test -p maverick-core` | ❌ Wave 0 |
| RADIO-04 | hardware-registry.toml parses as valid TOML and contains RAK entry | manual/doc | — | ❌ Wave 0 (file creation) |
| CORE-04 | hardware-registry.toml bundled in release archive | CI packaging | release.yml archive verification | ✅ (add file to dist/ copy) |

### Sampling Rate
- **Per task commit:** `cargo test --workspace` (x86, no spi feature)
- **Per wave merge:** `cargo test --workspace` + `cargo clippy --all-targets -- -D warnings`
- **Phase gate:** Full suite green before `/gsd-verify-work`

### Wave 0 Gaps
- [ ] `crates/maverick-adapter-radio-udp/src/uplink_source.rs` — `GwmpUdpUplinkSource` impl + unit tests
- [ ] `crates/maverick-core/src/ports/uplink_source.rs` — `UplinkSource` trait
- [ ] `crates/maverick-adapter-radio-spi/` — new crate with `build.rs`, vendored libloragw, `SpiUplinkSource`
- [ ] `crates/maverick-core/src/lns_config.rs` — `RadioConfig`, `RadioBackend` addition + deserialization unit tests
- [ ] `hardware-registry.toml` — initial file with RAK entry

---

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | — |
| V3 Session Management | no | — |
| V4 Access Control | no | — |
| V5 Input Validation | yes | Validate `spi_path` is a valid `/dev/spidev*` path; reject traversal attempts |
| V6 Cryptography | no | — |

### Known Threat Patterns for SPI + Config

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Path traversal via `spi_path` config field | Tampering | Validate `spi_path` starts with `/dev/spidev`; reject values with `..` or other suspicious patterns |
| Operator-supplied `spi_path` pointing to arbitrary character device | Elevation of Privilege | Document that `maverick-edge` must run as a user in the `spi` group (not root); reject if path is not owned by group spi |

---

## Sources

### Primary (HIGH confidence)
- `crates/maverick-adapter-persistence-sqlite/src/persistence/mod.rs` — `run_blocking` / `spawn_blocking` pattern verified by direct read
- `crates/maverick-runtime-edge/src/ingest/gwmp_loop.rs` — ingest loop structure verified by direct read
- `crates/maverick-core/src/ports/uplink_ingress.rs` — existing port trait pattern verified
- `crates/maverick-core/src/lns_config.rs` — `#[serde(default)]` patterns verified
- `.github/workflows/release.yml` — ARM cross-compile toolchain and sysroot env vars verified
- `.github/workflows/ci.yml` — `--all-features` in clippy lint job verified
- `doc.rust-lang.org/cargo/reference/features.html` — `dep:` optional dependency pattern, `CARGO_FEATURE_*` env vars

### Secondary (MEDIUM confidence)
- `github.com/lora-net/sx1302_hal` — libloragw C HAL function signatures (`lgw_start`, `lgw_stop`, `lgw_receive`, `lgw_pkt_rx_s` struct fields) [CITED: GitHub]
- `github.com/chirpstack/chirpstack-concentratord` — in-tree vendored libloragw + `cc` build pattern used in production [CITED: GitHub]
- `github.com/helium/concentrate` — Rust FFI bindings for libloragw-sx1302 (confirms `lgw_pkt_rx_s` fields) [CITED: GitHub]
- `doc.rust-lang.org/cargo/reference/build-script-examples.html` — `CARGO_FEATURE_*` build.rs pattern [CITED: official Cargo docs]

### Tertiary (LOW confidence)
- `loragw-hal` crate on crates.io: **NOT FOUND** — confirmed absent via crates.io and lib.rs search 2026-04-16

---

## Metadata

**Confidence breakdown:**
- Standard stack: MEDIUM — `loragw-hal` absence confirmed; in-tree pattern derived from ChirpStack evidence
- Architecture: HIGH — UplinkSource trait and spawn_blocking pattern verified against existing codebase
- Pitfalls: MEDIUM — bindgen ARM padding issue cited from GitHub issue; other pitfalls from reasoning about libloragw's C global state design
- CI cross-compile: HIGH — existing release.yml already handles ARM sysroot; extending for C sources is straightforward

**Research date:** 2026-04-16
**Valid until:** 2026-07-16 (stable dependencies; libloragw C HAL is mature and not fast-moving)

---

## RESEARCH COMPLETE
