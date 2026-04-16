# ARCHITECTURE RESEARCH — Maverick LNS
_Generated: 2026-04-16_

## Summary

Maverick already has the right structural instincts: hexagonal core, port/adapter separation, process-isolated TUI. The work ahead is not a redesign — it is completing the pattern in five areas where the current implementation is either absent (SPI adapter, extension IPC, offline buffering) or present but fragile (session state, FCnt). Each area below is analyzed against the existing codebase so recommendations are concrete rather than generic.

Confidence assessment: HIGH for areas derived directly from the codebase and established Rust/embedded patterns; MEDIUM for LoRaWAN HAL-specific SPI interface details (no web search access; based on Semtech HAL v2 design knowledge as of August 2025).

---

## Radio Backend Abstraction

### Current State

The existing port split is already correct. `UplinkIngressBackend` (in `maverick-core::ports::uplink_ingress`) is an identity trait whose `kind()` returns `UplinkBackendKind::GwmpUdp`. The actual receive loop lives in the composition root (`gwmp_loop.rs`) and calls `parse_push_data()` directly. The port trait is a marker only — it does not own the read loop.

This is the right separation for UDP because the UDP socket naturally produces a stream of datagrams. However, it creates a structural gap when a SPI concentrator is added: the SPI path produces frames through a blocking C FFI call into libloragw (Semtech HAL v2), not through a Rust async stream.

### The SPI Adapter Problem

The Semtech SX1302/SX1303 HAL v2 (`libloragw`) exposes:

- `lgw_start()` — initialises SPI bus, loads firmware.
- `lgw_receive(nb_pkt_max, rxpkt)` — blocking poll; returns 0..N packets per call.
- `lgw_stop()` — tears down the concentrator.

This is a C library, not a file descriptor. It cannot be given to `tokio::net::UdpSocket` style async polling. The correct Rust approach is `tokio::task::spawn_blocking` wrapping the poll loop, surfacing packets over an `mpsc` channel to the async ingest loop. This is the same pattern `SqlitePersistence` already uses for rusqlite.

### Recommended Abstraction

**Do not try to unify UDP and SPI behind a single streaming trait at the port level.** The read mechanics are too different (epoll-ready FD vs blocking C poll). Instead, keep `UplinkIngressBackend` as a marker identity trait (it already serves this purpose well) and introduce a second, optional port for the ingest source:

```rust
// In maverick-core::ports (new file: uplink_source.rs)
#[async_trait]
pub trait UplinkSource: Send + Sync {
    /// Receive the next batch of uplink observations. Returns Ok(None) when the
    /// source has been gracefully shut down. Never blocks the async executor.
    async fn next_batch(&self) -> AppResult<Option<Vec<UplinkObservation>>>;
}
```

Both `GwmpUdpSource` and `SpiConcentratorSource` implement this trait. The composition root (`gwmp_loop.rs`, or a new `ingest_loop.rs` that replaces it) becomes:

```rust
loop {
    let Some(batch) = source.next_batch().await? else { break };
    for obs in batch {
        ingest_uplink_with_lns_guard(&store, &svc, obs).await;
    }
}
```

The `GwmpUdpSource` implementation wraps the existing socket receive. The `SpiConcentratorSource` implementation:

1. Lives in a new crate: `maverick-adapter-radio-spi`.
2. Spawns a `tokio::task::spawn_blocking` task on construction.
3. The blocking task loops: `lgw_receive` → send results over `tokio::sync::mpsc::Sender<Vec<UplinkObservation>>`.
4. `next_batch()` calls `receiver.recv().await`.

The new `UplinkBackendKind::SpiConcentrator` variant is added to the enum without breaking existing JSON output.

### Build-Time Feature Flag

SPI concentrator support requires linking `libloragw` (a C library, Linux-only, not available on cross-compile hosts without the sysroot). Isolate this behind a Cargo feature flag on `maverick-adapter-radio-spi`:

```toml
[features]
default = []
spi-hal = ["dep:libloragw-sys"]  # binds libloragw FFI
```

The edge runtime opts in only when explicitly asked, keeping the default build clean and the UDP-only path testable anywhere.

### Hardware Compatibility Registry

The PROJECT.md lists "hardware compatibility registry — community-maintained list of tested hardware" as active scope. This is not an in-process concern. Implement it as a versioned TOML file (`hardware-compat.toml`) published alongside the release, containing tested `(board, concentrator_chipset, arch)` triples. The edge runtime `probe` command reads this file and compares against the current hardware probe output. No database, no network call; the file ships with the binary and is updated per release.

---

## Extension / Plugin Isolation

### Current State

The TUI (`maverick-extension-tui`) already demonstrates the correct model: it is a separate binary that knows nothing about the adapter crates and communicates with `maverick-edge` by spawning subprocesses and reading JSON from stdout. The pattern is proven and simple. `edge_runner.rs` confirms this — it is 90 lines of `std::process::Command` orchestration.

### What the Extension IPC Surface Needs to Provide

The TUI model works when the extension drives requests (ask `maverick-edge status`, get JSON back). It does not work for extensions that need to *receive* events from the core (e.g., an MQTT forwarder that must be notified when a new uplink arrives). The missing piece is a push/subscribe path.

### Recommended IPC Architecture: Local HTTP + SSE

Use a local HTTP server embedded in `maverick-edge` as the extension IPC boundary. This is the simplest option that satisfies all constraints:

- **Pull (request/response):** Extensions call `GET /api/v1/uplinks/recent`, `GET /api/v1/devices`, etc. Works today via `curl`/`reqwest`.
- **Push (event stream):** Extensions subscribe to `GET /api/v1/events/stream` — a Server-Sent Events (SSE) endpoint that emits newline-delimited JSON for each uplink accepted, session updated, etc.
- **Process isolation:** The HTTP server is a tokio task inside `maverick-edge`. Extensions crash independently. The core does not hold references to extension processes.
- **No additional runtime:** No message broker, no Unix socket protocol, no Protobuf schema compilation step.

The HTTP server (use `axum` — it is already in the Rust embedded ecosystem and integrates with tokio) binds on `127.0.0.1:17001` by default. The bind address is configurable via `lns-config.toml`.

Extensions discover the API address via `MAVERICK_API_ADDR` environment variable set by the TUI (or `maverick-edge` itself on startup) or by a well-known default.

### Fault Isolation Guarantees

- The HTTP server task panicking must not kill the ingest loop. Isolate them with separate `tokio::task::spawn` handles; a panic in one task does not propagate to others in tokio.
- Use `tokio::task::JoinHandle::abort()` + a supervisor task to restart the API server if it fails, without restarting the ingest loop.
- The SSE event channel should be bounded (`tokio::sync::broadcast` with a capacity of ~1000 events). If no subscriber is listening the channel is simply never consumed. If the channel is full (slow subscriber), the core drops the broadcast message and logs a warning. The core never blocks waiting for an extension to consume events.

### Extension Registration

For v1, extensions are not registered with the core — they are just processes that call the API. The `enabled_extensions` field already exists in `TuiConfig`. The TUI remains the orchestrator that starts extension processes.

---

## Session & Device State

### Current State

The `sessions` table is keyed by `dev_addr` (32-bit integer). Each row is a `SessionSnapshot`: one session per device address. The `lns_devices` table mirrors the declarative config and contains activation keys. There is no runtime join between them during ingest — `IngestUplink` only reads `sessions`.

The `DeviceRepository` port is defined (`exists(dev_eui) -> bool`) but has no adapter implementation.

### FCnt 32-bit Fix

The FCnt bug is in `parse_lorawan_payload` in `gwmp.rs`:

```rust
let fcnt = u16::from_le_bytes([raw[FCNT_START], raw[FCNT_END - 1]]) as u32;
```

This casts a 16-bit parse to u32 but loses the upper 16 bits. The LoRaWAN 1.0.x spec defines the frame counter as a 16-bit wire field, but sessions must maintain a 32-bit logical counter with implicit rollover detection (the server infers the upper 16 bits by assuming the device counter rolled over when the wire counter is much smaller than the session counter).

Fix: keep the wire parse as `u16`, but extend to 32-bit in `IngestUplink::execute()` using the current session's counter:

```rust
fn extend_fcnt(wire_fcnt: u16, session_fcnt: u32) -> u32 {
    let upper = session_fcnt & 0xFFFF_0000;
    let candidate = upper | (wire_fcnt as u32);
    if candidate < session_fcnt {
        candidate.wrapping_add(0x0001_0000) // rollover
    } else {
        candidate
    }
}
```

This logic belongs in the `ProtocolCapability::validate_uplink` implementation or a helper called from it, not in the parser. The parser correctly extracts what the wire says; the session context is needed to interpret it.

Store the 32-bit counter in `sessions.uplink_fcnt` (already `INTEGER NOT NULL` — SQLite integers are 64-bit, no schema change needed).

### MIC Verification

`parse_lorawan_payload` currently strips the MIC from the payload slice but does not verify it. The MIC is CMAC-AES128 over `B0 || MHDR || FHDR || FPort || FRMPayload` using the NwkSKey. The NwkSKey is stored in `lns_devices.nwks_key`.

Architecture implication: MIC verification requires joining `sessions` (to get DevAddr→DevEui mapping) and `lns_devices` (to get the NwkSKey). Currently `IngestUplink` only holds a `SessionRepository`. To verify MIC, either:

1. Add `NwkSKey` to `SessionSnapshot` so `SessionRepository::get_by_dev_addr` returns it — simplest; keeps one query path. Downside: the snapshot carries key material.
2. Add a `DeviceKeyRepository` port that `IngestUplink` calls after session lookup — cleaner separation, one additional async call per uplink.

**Recommendation:** Option 1 for v1. The session and its NwkSKey are inseparable for MIC verification; carrying the key in the snapshot is not a leak, it is the correct data colocation for the use case. The `lns_devices` adapter implementation populates it from the existing `nwks_key BLOB` column.

### Device Registry Implementation

`DeviceRepository` (`exists(dev_eui)`) is a single boolean check. Implement it directly on `SqlitePersistence` as:

```sql
SELECT COUNT(1) FROM lns_devices WHERE dev_eui = ?1
```

This unblocks the `DeviceRepository` port gap listed in the current architecture docs. No new table or schema change needed.

### Session Lifecycle

For v1 (ABP only), sessions are permanent — they never expire naturally. The only lifecycle events are:
- Created: via `config load` or `approve-device`.
- Updated: `uplink_fcnt` incremented on each accepted uplink.
- Deleted: explicitly via future `reject-device` or `config reload` removing a device.

For OTAA (v2 scope), sessions are created on JoinAccept and re-keyed on each rejoin. The `sessions` schema already has `dev_eui` — OTAA would reuse the same table with a new row per session epoch.

---

## Offline Buffering

### Current State

SQLite WAL mode is already enabled (`PRAGMA journal_mode = WAL`). Every accepted uplink is persisted atomically before the ingest loop moves to the next datagram. There is no in-memory queue between radio receive and SQLite write. This is good: the store-and-forward durability requirement is met for the ingest path.

The gap is the *output* path: there is no mechanism to deliver persisted uplinks to extensions or cloud sync. The `SyncBatchEnvelopeV1` type exists but is never populated or dispatched.

### Pattern: WAL Cursor (Output Buffering)

For any push-based output (cloud sync, MQTT, webhook), use a cursor over the `uplinks` table rather than a separate queue:

1. Each output extension (or the sync engine) registers a named cursor in a `sync_cursors` table:
   ```sql
   CREATE TABLE IF NOT EXISTS sync_cursors (
       name TEXT PRIMARY KEY NOT NULL,
       last_uplink_id INTEGER NOT NULL DEFAULT 0
   );
   ```

2. On each sync cycle, the extension reads:
   ```sql
   SELECT id, dev_addr, f_cnt, payload, application_id
   FROM uplinks
   WHERE id > (SELECT last_uplink_id FROM sync_cursors WHERE name = ?1)
   ORDER BY id ASC
   LIMIT 100;
   ```

3. After successful delivery, it advances the cursor:
   ```sql
   UPDATE sync_cursors SET last_uplink_id = ?1 WHERE name = ?2;
   ```

This is a WAL-cursor (also called a "bookmark" or "watermark") pattern. It is idiomatic for SQLite-backed store-and-forward systems:
- No separate queue table; the `uplinks` table is the queue.
- Cursor is durable; the extension can crash and resume from the last acked position.
- Multiple independent extensions have independent cursors.
- The pruning logic already in the codebase (`prune_uplinks_sql`) must never prune rows that have not been consumed by all registered cursors. Add a `MIN(last_uplink_id)` check in the prune query.

### Extension-Side Buffering

For the initial SSE-based extension IPC, the extension process receives events over SSE. If the extension crashes and reconnects, it has missed events. The recovery path is: on reconnect, call `GET /api/v1/uplinks/recent?since_id={last_seen}` to catch up before subscribing to live events. The extension owns its own `last_seen` cursor (persisted in its own state file or SQLite).

This means the core API must support `since_id` pagination on the uplinks endpoint — a trivial addition using the `uplinks.id` autoincrement primary key.

### SQLite Concurrency

`SqlitePersistence` currently uses a single `Mutex<Connection>`. This is correct for the ingest path (one writer). For the output/read path (extensions reading cursors), concurrent readers are safe in WAL mode because WAL allows one writer + many readers simultaneously. However, the `Mutex<Connection>` serialises all access including reads. For v1, this is acceptable given the low throughput target. For v2, consider a `r2d2` or `sqlx` connection pool where readers get their own read-only connections. Do not change this in v1.

---

## Build Order Implications

The five active items from PROJECT.md have hard dependencies. The correct build order, derived from what each item requires:

### Phase 1: Core Protocol Hardening (no new architecture needed)

1. **FCnt 32-bit fix** — change in `gwmp.rs` (parser) + `lorawan_10x_class_a.rs` (validation). No new ports, no new crates. Prerequisite for MIC verification (MIC uses the full 32-bit counter in the B0 block).
2. **MIC verification** — add `NwkSKey` to `SessionSnapshot`, implement CMAC-AES128 in `LoRaWAN10xClassA::validate_uplink`. Requires FCnt fix to be correct first. No new architecture.

These two can land in a single phase. They touch: `maverick-domain::session`, `maverick-core::protocol`, `maverick-adapter-persistence-sqlite::repos` (populate NwkSKey in session load), `maverick-adapter-radio-udp::gwmp` (parser cleanup only).

### Phase 2: Extension IPC Boundary

3. **Extension IPC boundary** — embed `axum` HTTP server in `maverick-runtime-edge`. Exposes pull API + SSE push. Requires Phase 1 to be complete so the data the API serves is trustworthy (MIC-verified uplinks). Adds `sync_cursors` table to SQLite schema.

This phase also implements `DeviceRepository` on `SqlitePersistence` (trivial; unblocks the port gap).

### Phase 3: Process Supervision

4. **Process supervision and self-healing** — implement a supervisor task inside `maverick-runtime-edge` that monitors the ingest loop tokio task and restarts it on failure. Also monitors the HTTP API task. This is built after the API exists because the supervisor needs to restart both.

The supervisor pattern: use `tokio::select!` with `JoinHandle` completion, re-spawn on unexpected exit. Consider `systemd` socket activation as an alternative for production deployments (the process is already a single binary; `systemd` handles restarts). The recommendation is to support both: internal tokio restart for transient panics, systemd for process-level restarts.

### Phase 4: SPI Radio Adapter

5. **Direct SPI radio adapter** — new crate `maverick-adapter-radio-spi`, implements `UplinkSource` (which must be defined in Phase 2 or earlier). Requires: `UplinkSource` port trait exists, build feature flag infrastructure, and the hardware compatibility registry file format is defined.

SPI adapter is last because it requires: Phase 1 (correct protocol handling for raw frames), the new `UplinkSource` abstraction, and cannot be integration-tested without physical hardware.

### Summary Dependency Graph

```
Phase 1: FCnt fix → MIC verification
              ↓
Phase 2: Extension IPC boundary (+ DeviceRepository impl, sync_cursors, UplinkSource trait)
              ↓
Phase 3: Process supervision
              ↓
Phase 4: SPI radio adapter (uses UplinkSource from Phase 2)
```

TUI device management (add/edit/remove devices via TUI backed by SQLite) can proceed in parallel with Phase 2 — it only requires that `maverick-edge config` subcommands exist (they do) and that the SQLite schema is stable (it is).

UDP surface hardening (bind to localhost or a configurable interface) is a single-line change to the bind address in `cli_constants.rs` and can be done in any phase.

---

## Gaps / Unknowns

**SPI HAL v2 build complexity (MEDIUM confidence)**
The Semtech `libloragw` C library requires a cross-compilation sysroot and specific linker flags for armv7/aarch64. The exact `build.rs` configuration for binding it in Rust is not verified here. Projects like `lora-phy` (embedded-hal based) and `chirpstack-concentratord` have done this, but their build configurations differ. This needs a proof-of-concept compile on the Pi target before committing to the adapter design.

**Region inference is broken for overlapping frequencies**
`infer_region()` in `gwmp.rs` has overlapping match arms: 902–928 MHz matches both `Us915` and `Au915`; 920–925 MHz can match `As923` or `Us915` or `Au915`. The ARCHITECTURE.md already flags this. The correct fix is to require the region to be declared in `lns-config.toml` and injected into the `UplinkObservation` from the config rather than inferred from frequency. This is a protocol-hardening concern that should be addressed in Phase 1.

**SQLite concurrent reader contention under extension load**
In WAL mode with a single `Mutex<Connection>`, read-heavy extensions (pulling uplink history) will serialise against the ingest writer. On a Raspberry Pi under sustained sensor traffic this may cause ingest latency. Measure first; do not prematurely optimise. If contention appears, open a second read-only connection for the API server path.

**Extension IPC authentication**
The local HTTP API will bind only on `127.0.0.1`. For v1, no authentication is planned — local processes are trusted. If extensions are ever allowed to run on a different host (e.g., a remote dashboard), mutual TLS or token auth is required. This is a v2 concern but the API surface design should not make it hard to add.

**`DeviceRepository` port is too minimal**
`DeviceRepository::exists(dev_eui)` is the only method defined. For MIC verification, NwkSKey lookup, and future OTAA, the repository needs `get_by_dev_eui(dev_eui) -> Option<DeviceRecord>` returning key material. Expand the trait before implementing the adapter to avoid a later breaking change at the port boundary.

**`HybridRetentionDefaults` constructors are vestigial**
Flagged in the existing architecture docs. The three constructors (`constrained`, `balanced`, `high_capacity`) return identical values. This means the storage pressure and pruning behaviour does not vary with install profile as intended. Should be fixed in Phase 1 alongside FCnt to ensure the constrained Pi profile actually prunes more aggressively.
