# CONCERNS — Maverick Codebase Map
_Generated: 2026-04-16_

## Summary

Maverick is a young (v0.1.4) edge LoRaWAN runtime with a clean hexagonal architecture and good test coverage for its core use case. The most significant risks are protocol-level: no MIC (Message Integrity Code) verification is performed on uplinks, meaning any device can replay or forge frames against any known DevAddr. Secondary concerns involve the use of `process::exit` throughout the CLI layer (skips Drop), a single-connection SQLite lock under async workloads, and several incomplete placeholder features documented in the codebase but not yet implemented (cloud sync, `recent-errors`, Class B/C devices).

---

## Critical Concerns

### C1 — No MIC verification on uplink frames
**Files:** `crates/maverick-adapter-radio-udp/src/gwmp.rs:128–162`, `crates/maverick-core/src/protocol/lorawan_10x_class_a.rs:29–48`

The GWMP parser extracts DevAddr, FCnt, FPort, and FRMPayload from raw LoRaWAN frames but **never verifies the 4-byte MIC** (Message Integrity Code). The tail 4 bytes are stripped by `parse_lorawan_payload` (line 155–160) without validation. The `validate_uplink` protocol method only checks region, session existence, device class, and that `f_cnt > last_f_cnt`. Any attacker on the same LAN segment who knows a valid DevAddr and sends a frame with a monotonically increasing FCnt will be accepted and persisted as a legitimate uplink. The ABP session keys (`apps_key`, `nwks_key`) are stored in `lns_devices` but are never loaded or used for any crypto operation.

**Impact:** All uplink authentication is bypassed. This is a deliberate V1 limitation per code comments ("Optional ABP session keys; not required for ingest until downlink/crypto is wired") but must be clearly gated before any production deployment.

**Fix approach:** Wire `NwkSKey` through `ProtocolContext`, compute AES-CMAC over the frame header+payload, and compare against the last 4 bytes before calling `ProtocolDecision::Accept`.

---

### C2 — FCnt is only 16 bits wide in the GWMP parser but stored as 32 bits
**File:** `crates/maverick-adapter-radio-udp/src/gwmp.rs:139–140`

```rust
let fcnt =
    u16::from_le_bytes([raw[LORAWAN_FHDR_FCNT_START], raw[LORAWAN_FHDR_FCNT_END - 1]]) as u32;
```

The LoRaWAN spec uses a 16-bit over-the-air FCnt with a 32-bit server-side counter maintained by incrementing the upper 16 bits on wrap. The current code parses only the lower 16 bits and casts to u32; the upper 16 bits are always zero. Once a device wraps its FCnt past 0xFFFF the ingest will reject every subsequent frame as a duplicate (because `obs.f_cnt <= session.uplink_frame_counter`), permanently bricking the session without any rollover logic.

**Impact:** Devices with more than 65535 uplinks silently stop being ingested. No error is surfaced beyond a `rejected:RejectDuplicateFrameCounter` audit entry.

**Fix approach:** Track and reconstruct the upper half from session state (standard LNS approach), or at minimum reject with a distinct error so operators can diagnose the session.

---

### C3 — UDP socket binds to `0.0.0.0:17000` by default with no authentication
**Files:** `crates/maverick-runtime-edge/src/cli_constants.rs`, `crates/maverick-extension-tui/src/config.rs:14`

The default GWMP bind is `0.0.0.0:17000`. GWMP/UDP has no built-in authentication; any host that can reach this port can inject frames. On a Raspberry Pi connected to the internet (typical edge deployment) this means the ingest loop accepts arbitrary LoRaWAN-shaped datagrams from any source IP.

**Impact:** Combined with C1 (no MIC), this is an open uplink injection vector. The autoprovision path (`lns_guard.rs`) will create pending rows for any unseen DevAddr at up to 10 per gateway per minute by default, allowing storage exhaustion via crafted datagrams at a moderate rate.

**Fix approach:** Document clearly that GWMP must be firewalled to localhost only. Consider defaulting bind to `127.0.0.1:17000`. The rate limiter in `lns_guard.rs` mitigates but does not prevent flooding.

---

## Moderate Concerns

### M1 — `std::process::exit` called throughout CLI handlers; Drop implementations skipped
**Files:** `crates/maverick-runtime-edge/src/commands/config.rs` (lines 57, 64, 73, 85, 94, 108, 114, 120, 131, 148, 159, 164, 172, 178, 183, 190, 196, 258, 265, 274, 285, 295, 303, 311), `crates/maverick-runtime-edge/src/commands.rs` (lines 51, 55, 89, 95)

`std::process::exit(1)` is used directly in ~25 CLI error paths rather than propagating errors and letting `main` decide. This means SQLite's WAL checkpoint, any in-flight `spawn_blocking` tasks, and OS-level file flushes may not complete cleanly on error. Rust's `Drop` implementations are bypassed.

**Impact:** On a write-heavy operation like `config load` that's partway through a transaction, a kill signal or error exit is less likely to corrupt SQLite due to WAL atomicity, but accumulated dirty pages may not be checkpointed. More importantly it makes testability of CLI commands difficult.

**Fix approach:** Return `Result<(), String>` from command handlers and call `std::process::exit` only at the `main` boundary.

---

### M2 — Single shared `Mutex<Connection>` serializes all SQLite I/O
**File:** `crates/maverick-adapter-persistence-sqlite/src/persistence/mod.rs:52`

```rust
pub(super) conn: std::sync::Mutex<Connection>,
```

All reads and writes (sessions, uplinks, audit, config, pressure snapshot) share one `std::sync::Mutex<Connection>`. Every async path calls `run_blocking` which dispatches to `spawn_blocking`. Under load this creates a thundering herd on the lock and wastes Tokio thread-pool threads waiting for the mutex rather than doing real work. The busy-retry layer in `busy.rs` adds additional latency on top.

**Impact:** At very low message rates (typical edge) this is fine. Any burst (many devices, or supervised `ingest-loop` processing many datagrams rapidly) will serialize all writes and degrade latency significantly. The pressure snapshot path (`pressure_snapshot_blocking`) also acquires the same lock, meaning a health check can block ingest.

**Fix approach:** Pool read-only connections separately from the writer, or use `rusqlite`'s connection pool via `r2d2-sqlite`. At minimum, separate the pressure snapshot read connection.

---

### M3 — `expect("validated lns config")` panics in `apply_lns_config_inner` defeat the error contract
**File:** `crates/maverick-adapter-persistence-sqlite/src/persistence/lns_ops.rs:288–400`

The internal function `apply_lns_config_inner` contains 12+ `.expect("validated lns config")` and `.expect("validated")` calls on hex-parsing operations. These are guarded by a prior call to `doc.validate()` at line 21, creating an implicit assumption that validation is always called first. However, `apply_lns_config_inner` is a private function and the guarantee is non-local. If validation is ever skipped (e.g., via a future refactor calling the inner function directly), this panics in a blocking task, poisoning the Mutex and bricking the persistence layer for the process lifetime.

**Impact:** A panic in `spawn_blocking` or a synchronous call inside the mutex will poison it. `run_with_busy_retry` already handles `Mutex::lock` returning `Err(_)` by returning `AppError::Infrastructure`, but all subsequent operations on that `SqlitePersistence` instance will then always fail.

**Fix approach:** Return `Result` from `apply_lns_config_inner`, propagate errors, and remove the `.expect` calls.

---

### M4 — `recent-errors` command is a stub; no structured log persistence exists
**Files:** `crates/maverick-runtime-edge/src/cli_constants.rs` (constant `RECENT_ERRORS_NOT_WIRED_MESSAGE`), `crates/maverick-runtime-edge/src/commands.rs:201–210`

The `recent-errors` command returns a hardcoded stub response indicating that log tailing is not yet wired. There is no log file, ring buffer, or structured error sink beyond the audit table in SQLite. Operators relying on this for diagnostics get no actionable output.

**Impact:** Debugging production issues requires access to stderr/journald logs directly; the CLI surface is incomplete. Documented as a "v1 placeholder" in command help text ("placeholder" appears in the CLI doc comment).

---

### M5 — `infer_region` from frequency has overlapping and ambiguous ranges
**File:** `crates/maverick-adapter-radio-udp/src/gwmp.rs:164–173`

```rust
fn infer_region(freq_mhz: Option<f64>) -> RegionId {
    match freq_mhz {
        Some(v) if (863.0..=870.0).contains(&v) => RegionId::Eu868,
        Some(v) if (902.0..=928.0).contains(&v) => RegionId::Us915,
        Some(v) if (915.0..=928.0).contains(&v) => RegionId::Au915,   // shadowed by Us915
        Some(v) if (920.0..=923.5).contains(&v) => RegionId::As923,   // shadowed by Us915
        Some(v) if (923.0..=925.0).contains(&v) => RegionId::As923,   // also shadowed
        _ => RegionId::Eu868,                                           // silent default
    }
}
```

AU915 (915–928 MHz) and AS923 (920–923.5 MHz) are both entirely contained within the US915 match arm (902–928 MHz). They can never be reached. Additionally, when `freq` is `None` (field missing from GWMP JSON), the function silently defaults to `Eu868` rather than returning an error or unknown region.

**Impact:** Devices configured as AU915 or AS923 will have their region misidentified as US915. The region mismatch will then cause `validate_uplink` to return `RejectRegionMismatch` if the session has the correct region, silently dropping all their uplinks.

**Fix approach:** Re-order match arms from most-specific to least-specific, or use explicit frequency channel plans per region. Return a `Result` when `freq` is absent rather than defaulting.

---

### M6 — Rate-limit bucket uses process-global static; survives across logical test boundaries
**File:** `crates/maverick-runtime-edge/src/ingest/lns_guard.rs:21–36`

```rust
static BUCKET: OnceLock<Mutex<HashMap<GatewayMinuteKey, u32>>> = OnceLock::new();
```

The autoprovision rate-limit state is stored in a `OnceLock`-backed static. This is shared across all test cases in the same process and across all concurrent ingest workers. Rate limits accumulated by one test cannot be cleared by another, making tests that rely on rate-limiting non-deterministic when run in parallel or sequentially without process restart.

**Impact:** Test isolation issue; could cause intermittent test failures. In production, a process restart resets all rate-limit state, which may be intentional, but it is undocumented.

---

### M7 — Cloud sync is entirely unimplemented; `maverick-cloud-core` is a one-trait stub
**File:** `crates/maverick-cloud-core/src/lib.rs`

The `HubSyncIngest` trait body contains a doc comment "implementation in v1.x". The `SyncBatchEnvelopeV1` struct in `maverick-extension-contracts` has no callers that populate it with real data. There is no edge-to-hub sync path, no store-and-forward queue, and no dedup logic.

**Impact:** If cloud sync is mentioned in user-facing documentation or marketing, it must be clearly gated as not-yet-implemented. The extension contracts create a stable wire format but nothing produces or consumes it.

---

## Minor / Tech Debt

### T1 — `expect` calls in test helper code will produce confusing panics on CI
**Files:** `crates/maverick-adapter-radio-udp/src/gwmp.rs:187,210`, `crates/maverick-adapter-radio-udp/src/resilient.rs:349`, `crates/maverick-adapter-radio-udp/src/udp_downlink.rs:60,61,65,71,77,79,87,88,97,109`

Several `expect()` calls exist inside test helpers and test setup code. These are acceptable in tests but the ones in `udp_downlink.rs` are in the production implementation body (`bind_ephemeral`, `recv_and_ack`). If `UdpSocket::bind` fails (port already in use, permission denied), the process panics rather than returning an error.

**Fix approach:** Return `AppResult` from `bind_ephemeral` and propagate errors instead of calling `.expect`.

---

### T2 — `db_file_bytes()` calls `std::fs::metadata` on every write, inside the SQLite lock
**File:** `crates/maverick-adapter-persistence-sqlite/src/persistence/mod.rs:83–87`, `crates/maverick-adapter-persistence-sqlite/src/persistence/pruning.rs:63`

`prune_hard_limit_circular_sql` calls `self.db_file_bytes()` which does a filesystem `stat` syscall. This happens after every write (after every append, upsert, and audit emit). The `db_file_bytes` call also occurs inside the async task that holds the Mutex, adding unnecessary syscall latency on the hot ingest path.

**Fix approach:** Cache the file size value and invalidate periodically, or sample it only on the pruning path at a low frequency.

---

### T3 — `apply_lns_config_inner` calls `prune_sessions_lru_sql` and `prune_hard_limit_circular_sql` on `conn` after `tx.commit()`
**File:** `crates/maverick-adapter-persistence-sqlite/src/persistence/lns_ops.rs:443–446`

After `tx.commit()` at line 443, the function calls pruning on the same `conn`. This means pruning happens in a separate implicit transaction after the config load commit. If the process dies between the commit and the pruning calls, the database is left with slightly more rows than the policy allows. This is unlikely to cause data loss but violates the atomicity intent.

---

### T4 — `hex_upper_8` in `lns_ops.rs` silently returns an error string for malformed data
**File:** `crates/maverick-adapter-persistence-sqlite/src/persistence/lns_ops.rs:201–209`

```rust
fn hex_upper_8(bytes: &[u8]) -> String {
    if bytes.len() != 8 {
        return format!("invalid_len_{}", bytes.len());
    }
```

When `dev_eui` bytes stored in SQLite have unexpected length, this returns a sentinel string `"invalid_len_N"` instead of propagating an error. This string will appear in CLI `list-devices` JSON output without any indication that the row is malformed.

---

### T5 — `run_radio_ingest_supervised` opens the SQLite database once before the receive loop
**File:** `crates/maverick-runtime-edge/src/ingest/gwmp_loop.rs:193–216`

The supervised ingest loop opens `SqlitePersistence` once at startup. If the database file is deleted or corrupted during the run (e.g., by a concurrent `config load` gone wrong), the loop continues with a stale handle. The WAL mode mitigates corruption, but there is no reconnect logic.

---

### T6 — `serde_json::to_string(&out).expect("ingest result")` repeated ~15 times
**Files:** `crates/maverick-runtime-edge/src/ingest/gwmp_loop.rs:46,65,78,99,137,163,188,212,295`, `crates/maverick-runtime-edge/src/commands.rs:139,198,208,222,231,241,254,261`

The pattern `serde_json::to_string(&out).expect("X json")` appears approximately 20 times across two files. Serializing a known-good struct to JSON should never fail, making these `expect` calls correct in practice, but the repetition is noisy. This could be extracted into a helper (e.g., `print_json_line`).

---

### T7 — No `updated_at_ms` index on `sessions` table; LRU pruning is a full table scan
**File:** `crates/maverick-adapter-persistence-sqlite/src/schema.sql`

The `sessions` table has `dev_addr` as primary key and no secondary index on `updated_at_ms`. The LRU pruning query (`sql_prune_sessions_lru`) orders by `updated_at_ms ASC` without an index, which becomes a full table scan as session count grows. For the constrained profile (max_records_critical ≈ low hundreds) this is fine; for the high-capacity profile with thousands of sessions it will slow every write.

**Fix approach:** Add `CREATE INDEX IF NOT EXISTS idx_sessions_updated_at ON sessions(updated_at_ms);` to `schema.sql`.

---

### T8 — `total_disk_bytes_hint` returns the first disk with non-zero space, not the disk containing the data dir
**File:** `crates/maverick-runtime-edge/src/probe.rs:63–66`

```rust
disks.iter().map(|d| d.total_space()).find(|t| *t > 0)
```

On a system with multiple disks (e.g., SD card + USB stick), this returns whichever disk is enumerated first with non-zero capacity. If the data directory is on `/var/lib/maverick` (typically the SD card) but the first enumerated disk is the USB stick, storage pressure ratios will be computed against the wrong capacity.

---

## Gaps / Unknowns

### G1 — No OTAA join procedure; OTAA devices cannot actually join
`ActivationMode::Otaa` is stored in `lns_devices` with `join_eui` and `app_key`, but there is no Join Request / Join Accept handler. OTAA devices listed in `lns-config.toml` only become functional if their `dev_addr` is also manually provided (pre-provisioned). The field is documented as "omit for OTAA until a session exists" but no mechanism populates the session automatically.

### G2 — `maverick-cloud-core` crate is a workspace member but has no reverse dependencies
The `maverick-cloud-core` crate is compiled as part of the workspace but nothing depends on it at runtime. It adds build time and artifact surface without delivering functionality. Its presence may confuse contributors about the v1 scope.

### G3 — `recent-errors` command stub message references "structured logs live on disk in full impl" but no log file path is defined
The in-code comment says logs will "live on disk in full impl" but there is no `log_path`, log rotation, or log directory constant defined anywhere in `maverick-runtime-edge`. If this feature is planned, its storage location and format should be specified before the next slice.

### G4 — `DeviceClass::ClassB` and `ClassC` exist in the domain model but are never handled
`crates/maverick-domain/src/session.rs:8–11` defines all three device classes. `LoRaWAN10xClassA::validate_uplink` returns `RejectUnsupportedClass` for anything other than ClassA. No other protocol capability module exists. Storing ClassB/C devices in config will result in silent perpetual rejection with no useful operator message.

### G5 — `UdpDownlinkTransport` is implemented and tested but never wired into the ingest loop
`crates/maverick-adapter-radio-udp/src/udp_downlink.rs` implements a UDP downlink sender. The `IngestUplink` use case has no `RadioTransport` port — downlink is disconnected from the ingest path. The `DownlinkProbe` CLI command exercises transport independently but there is no way to schedule or trigger a Class A Rx1/Rx2 downlink in response to an uplink.

### G6 — No schema migration for `lns_meta` when upgrading from pre-autoprovision databases
`migrate_legacy_columns` (`sql.rs:32–36`) adds `application_id` columns to old databases. `migrate_lns_devices_v2` handles the `activation_mode` column addition. However, there is no migration for `lns_meta` if it doesn't exist (it's created on first `config load`). A database from an installation that never ran `config load` before upgrading will have `lns_meta` absent; `read_lns_meta` returns defaults in that case, so behavior is correct, but this is implicit and undocumented.

---

_Concerns audit: 2026-04-16_
