# Codebase Concerns

**Analysis Date:** 2026-04-16

## Tech Debt

### Heavy use of `std::process::exit` bypassing Drop handlers

**Area:** CLI command handlers
**Files:**
- `crates/maverick-runtime-edge/src/commands/config.rs` (31 instances, lines 57, 62, 74, 84, 91, 96, 109, 116, 121, 131, 147, 159, 168, 175, 182, 189, 196, 226, 234, 241, 258, 264, 272, 284, 294, 300, 308, 320)
- `crates/maverick-runtime-edge/src/commands.rs` (5 instances, lines 54, 89, 97, 103, 105)
- `crates/maverick-runtime-edge/src/ingest/gwmp_loop.rs` (3 instances, lines 218, 239, 263)

**Issue:** `std::process::exit(1)` is called directly in ~39 error paths throughout the CLI layer. This bypasses Rust's `Drop` implementations, meaning:
- SQLite WAL checkpoint may not flush (`crates/maverick-adapter-persistence-sqlite/src/persistence/mod.rs:89-104` documents `close()` must be called before exit)
- In-flight `spawn_blocking` tasks are abandoned
- OS-level file flushes may not complete

**Impact:** On write-heavy operations (e.g., `config load`), a kill signal or error could leave WAL frames uncheckpointed. The `SqlitePersistence::close()` method exists specifically to address this (RELI-02), but is never called because CLI exits bypass Drop.

**Fix approach:** Return `Result<(), AppError>` from command handlers and call `std::process::exit` only at the `main` boundary after proper cleanup.

---

### Large file complexity — `lns_ops.rs` at 502 lines

**File:** `crates/maverick-adapter-persistence-sqlite/src/persistence/lns_ops.rs`

**Issue:** Single file contains LNS config sync, pending device management, application/device listing, and session management. The `apply_lns_config_inner` function (lines ~288-450) has 12+ `.expect("validated lns config")` calls that assume prior validation.

**Impact:** If validation is ever skipped via refactor, these `.expect` calls panic inside `spawn_blocking`, poisoning the SQLite Mutex and bricking persistence for the process lifetime.

**Fix approach:** Extract functions, propagate errors properly, remove `.expect` calls.

---

### Large file complexity — `ingest_uplink.rs` at 423 lines

**File:** `crates/maverick-core/src/use_cases/ingest_uplink.rs`

**Issue:** Contains session lookup, MIC verification, payload decryption, and uplink record creation in a single large module. Test mocks use `tokio::sync::Mutex` patterns that don't mirror production behavior.

**Impact:** Hard to test individual concerns in isolation. The `expect("session confirmed present before this point")` at line 167 creates an implicit assumption about prior validation.

---

### Large file complexity — `lns_config.rs` at 453 lines

**File:** `crates/maverick-core/src/lns_config.rs`

**Issue:** Declarative config parsing, TOML deserialization, and validation mixed together. Contains multiple `.expect("valid abp")` / `.expect("valid otaa")` / `.expect("spi path set")` calls (lines 342, 367, 451).

**Impact:** Validation failures in production will panic instead of returning user-friendly errors.

---

## Known Bugs

### Region inference has shadowed match arms — AU915/AS923 unreachable

**File:** `crates/maverick-adapter-radio-udp/src/gwmp.rs:164-173`

**Symptoms:** AU915 (915-928 MHz) and AS923 (920-923.5 MHz) are entirely within US915 (902-928 MHz). The US915 match arm `if (902.0..=928.0).contains(&v)` matches first, making AU915/AS923 unreachable.

**Trigger:** Devices configured with AU915 or AS923 regions send uplinks with frequencies in those ranges.

**Workaround:** None — these devices will be misidentified as US915 and rejected with `RejectRegionMismatch`.

---

### FCnt rollover not handled — devices brick after 65535 uplinks

**File:** `crates/maverick-adapter-radio-udp/src/gwmp.rs:139-140`

**Symptoms:** Only lower 16 bits of FCnt are parsed; upper 16 bits are always zero. Once `f_cnt > 0xFFFF`, every subsequent frame is rejected as `RejectDuplicateFrameCounter`.

**Trigger:** High-volume devices crossing the 64K uplink boundary.

**Workaround:** None — session becomes permanently non-functional.

---

### Zero-value default session keys on ABP device approval

**File:** `crates/maverick-adapter-persistence-sqlite/src/persistence/lns_ops.rs:95-96`

**Symptoms:** When approving a pending device, if no existing session is found, the code uses `unwrap_or([0u8; 16])` for both `nwk_s_key` and `app_s_key`. This means ABP devices without a prior session get all-zero session keys.

**Trigger:** Using `config approve-device` for an ABP device that has no existing session in the database.

**Workaround:** Ensure the device has already communicated (creating a session with proper keys) before approval.

---

## Security Considerations

### No MIC verification on uplink frames

**Files:**
- `crates/maverick-adapter-radio-udp/src/gwmp.rs` (MIC bytes stripped at line 155-160)
- `crates/maverick-core/src/protocol/lorawan_10x_class_a.rs` (validation only checks region/session/class/FCnt)

**Risk:** Any attacker on the same LAN segment who knows a valid DevAddr can forge uplinks with incrementing FCnt. Combined with C3 (UDP binding), this is an open injection vector.

**Current mitigation:** Autoprovision rate limiting (10/minute/gateway) in `lns_guard.rs` limits flood rate.

**Recommendations:**
- Document that GWMP/UDP must be firewalled to localhost only
- Default bind address should be `127.0.0.1:17000` (SEC-01 change noted in `cli_constants.rs:28`)
- Wire MIC verification before any production deployment

---

### Default GWMP bind changed from 0.0.0.0 to 127.0.0.1 (SEC-01)

**File:** `crates/maverick-runtime-edge/src/cli_constants.rs:28-32`

**Note:** The comment indicates this was a deliberate security fix. The default bind is now loopback-only, requiring explicit `0.0.0.0:17000` override for external packet forwarders.

---

### Unsafe code allowed with warnings only

**File:** `Cargo.toml:50`

**Issue:** `unsafe_op_in_unsafe_fn = "warn"` allows unsafe code blocks without failing CI. While no `unsafe` blocks were found in the codebase, the linter configuration permits them.

**Recommendations:** Consider upgrading to `unsafe_op_in_unsafe_fn = "deny"` if the project wants to eliminate unsafe code entirely.

---

## Performance Bottlenecks

### Single Mutex<Connection> serializes all SQLite I/O

**File:** `crates/maverick-adapter-persistence-sqlite/src/persistence/mod.rs:52`

**Problem:** All reads and writes share one `std::sync::Mutex<Connection>`. Under burst traffic, this creates a thundering herd on the lock.

**Impact:** Health checks (`pressure_snapshot_blocking`) can block ingest. High message rates will serialize writes and degrade latency.

**Improvement path:** Pool read-only connections separately from the writer, or use `r2d2-sqlite`.

---

### Filesystem stat on every write

**File:** `crates/maverick-adapter-persistence-sqlite/src/persistence/mod.rs:83-87`

**Problem:** `db_file_bytes()` calls `std::fs::metadata` inside the SQLite lock after every write to check storage pressure.

**Impact:** Adds syscall latency on the hot ingest path.

**Improvement path:** Cache file size and invalidate periodically or only on pruning paths.

---

### Rate-limit bucket uses process-global static

**File:** `crates/maverick-runtime-edge/src/ingest/lns_guard.rs:21-22`

**Problem:** `static BUCKET: OnceLock<Mutex<HashMap<GatewayMinuteKey, u32>>>` is shared across all concurrent ingest workers and test cases.

**Impact:** Rate limits accumulate across logical test boundaries, causing non-deterministic test behavior. In production, process restart resets all state.

---

## Fragile Areas

### `infer_region` silently defaults to EU868 for unknown frequencies

**File:** `crates/maverick-adapter-radio-udp/src/gwmp.rs:164-173`

**Why fragile:** When GWMP JSON lacks a `freq` field, the function returns `Eu868` silently instead of an error. This causes all uplinks with missing frequency to be processed as EU868, potentially misclassifying devices.

**Safe modification:** Change return type to `Result<RegionId, AppError>` and propagate missing frequency as an error.

---

### Session lookup assumes validation already occurred

**File:** `crates/maverick-core/src/use_cases/ingest_uplink.rs:167`

**Why fragile:** `let session = session.expect("session confirmed present before this point")` creates a non-local assumption that `validate_uplink` was called and succeeded. If called out of order, this panics.

**Safe modification:** Use `ok_or_else` with a descriptive error instead of `expect`.

---

### Supervised ingest loop has no database reconnection logic

**File:** `crates/maverick-runtime-edge/src/ingest/gwmp_loop.rs:193-216`

**Why fragile:** `SqlitePersistence` is opened once before the loop. If the database file is deleted or corrupted during the run, the loop continues with a stale handle. No reconnect logic exists.

**Safe modification:** Add periodic health checks and reconnection on errors.

---

## Scaling Limits

**Resource: Session count**

The LRU pruning query (`sql_prune_sessions_lru`) performs a full table scan because `updated_at_ms` has no index. For constrained profiles (hundreds of sessions), this is acceptable. High-capacity profiles with thousands of sessions will see degraded write performance.

**Resource: UDP packet rate**

Single-threaded GWMP receiver in `gwmp_loop.rs`. Very high packet rates (>1000/sec) will serialize in the single ingest loop. Consider parallelizing by gateway EUI if needed.

---

## Dependencies at Risk

**rusqlite 0.33 (bundled)**

The `rusqlite` crate with bundled SQLite is a single dependency for all persistence. While well-maintained, any SQLite issues affect the entire persistence layer. The `bundled` feature ensures a consistent SQLite version but may lag behind security patches in system SQLite libraries.

---

## Missing Critical Features

**Cloud sync not implemented**

`crates/maverick-cloud-core/src/lib.rs` contains only a stub trait (`HubSyncIngest`) with no implementation. `SyncBatchEnvelopeV1` in `maverick-extension-contracts` has no callers producing or consuming real data.

**No downlink scheduling**

`UdpDownlinkTransport` exists in `crates/maverick-adapter-radio-udp/src/udp_downlink.rs` but is never wired into the ingest path. Class A Rx1/Rx2 downlink cannot be triggered in response to uplinks.

**No OTAA join procedure**

`ActivationMode::Otaa` is stored but no Join Request/Join Accept handler exists. OTAA devices cannot complete the join flow automatically.

**recent-errors stub**

`crates/maverick-runtime-edge/src/cli_constants.rs:18` defines `RECENT_ERRORS_NOT_WIRED_MESSAGE`. No log file, ring buffer, or structured error sink exists beyond SQLite audit table.

---

## Test Coverage Gaps

**Untested: MIC verification failure path**

The `ingest_uplink.rs` module tests happy-path uplink processing but doesn't appear to test the MIC invalid rejection path with a live `IngestUplink` service (only unit tests with mock stores).

**Untested: Session rollover at FCnt boundary**

No test exercises the FCnt 0xFFFF → 0x10000 transition. The `RejectDuplicateFrameCounter` path is covered only by protocol-level unit tests, not end-to-end.

**Untested: Database recovery after panic**

If `spawn_blocking` panics (e.g., from `.expect` in lns_ops), the Mutex poisons. No test verifies recovery behavior after a poisoned persistence layer.

**Untested: Class B/C device handling**

`DeviceClass::ClassB` and `ClassC` exist in domain model but are never tested. `validate_uplink` returns `RejectUnsupportedClass` for them, but no integration test verifies operator experience when these devices are configured.

---

## Observations

**Good patterns:**
- No TODO/FIXME/HACK comments found — codebase appears well-maintained
- `thiserror` derive for error types (`crates/maverick-core/src/error.rs`)
- Circuit breaker pattern in `resilient.rs` with proper half-open state
- SQLite WAL mode provides atomic write safety
- `tracing` for structured logging in hot paths

**Patterns requiring attention:**
- 102 `expect()` / `unwrap()` calls across codebase (many in test code, but production code in `gwmp.rs`, `udp_downlink.rs` uses them in socket operations)
- `panic = "abort"` in release profile means any panic terminates immediately without stack unwinding
- Error messages use stringly-typed `AppError::Infrastructure(format!(...))` rather than structured variants

---

*Concerns audit: 2026-04-16*
