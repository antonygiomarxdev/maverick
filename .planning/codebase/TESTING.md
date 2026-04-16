# TESTING — Maverick Codebase Map
_Generated: 2026-04-16_

## Summary

Testing is split across two levels: inline `#[cfg(test)]` unit tests co-located with the source they test, and a dedicated `maverick-integration-tests` crate that exercises cross-crate composition with real SQLite and real UDP sockets. There is no separate E2E test suite; the integration tests are the highest-fidelity level. The framework is the standard Rust test harness with `#[tokio::test]` for async cases and `tempfile` for ephemeral database files.

---

## Test Types Present

| Type | Location | Scope |
|------|----------|-------|
| Unit tests | `#[cfg(test)] mod tests` at bottom of each source file | Single module / function in isolation |
| Integration tests | `crates/maverick-integration-tests/tests/*.rs` | Multi-crate composition against real adapters |

---

## Test File Locations and Naming

### Inline unit tests
Co-located at the bottom of the implementation file, inside `#[cfg(test)] mod tests { ... }`:

| File | What it tests |
|------|--------------|
| `crates/maverick-core/src/use_cases/ingest_uplink.rs` | `IngestUplink::execute` — happy path + f_cnt rejection |
| `crates/maverick-core/src/protocol/lorawan_10x_class_a.rs` | FCnt acceptance/rejection |
| `crates/maverick-core/src/lns_config.rs` | `LnsConfigDocument::validate` — ABP, OTAA, schema version |
| `crates/maverick-core/src/storage/policy.rs` | Profile/policy numeric assertions |
| `crates/maverick-adapter-radio-udp/src/gwmp.rs` | GWMP JSON parsing, multi-rxpk, malformed input |
| `crates/maverick-adapter-radio-udp/src/resilient.rs` | Circuit breaker: timeout, open, half-open, close |
| `crates/maverick-extension-tui/src/main.rs` | Default TUI config values |

### Integration test files
All live in `crates/maverick-integration-tests/tests/`:

| File | Focus area |
|------|-----------|
| `smoke.rs` | Domain roundtrips, serde, extension contracts |
| `persistence_sqlite.rs` | SQLite port adapter (sessions, uplinks, retention, concurrency) |
| `operator_local_gateway_e2e.rs` | Full ingest pipeline: GWMP parse → `IngestUplink` → SQLite persist → storage pressure |
| `radio_transport_resilience.rs` | `ResilientRadioTransport` over real UDP socket + circuit breaker cross-crate |

---

## Test Frameworks and Utilities

### Runner
Cargo's built-in test harness. No external test runner (no `nextest` config found).

### Async test attribute
`#[tokio::test]` from the `tokio` crate for all async test cases:
```rust
#[tokio::test]
async fn ingest_happy_path_updates_session_and_uplink() { ... }
```

### Synchronous tests
Plain `#[test]` for pure logic and synchronous SQLite operations:
```rust
#[test]
fn sqlite_ddl_defines_tables_matching_schema_names() { ... }

#[test]
fn validates_abp_device() { ... }
```

### `tempfile` for isolated SQLite files
Every integration test that needs a database creates an ephemeral temp directory:
```rust
let dir = tempfile::tempdir().expect("tempdir");
let db = dir.path().join("maverick.db");
let store = SqlitePersistence::open(&db, policy, SqlitePersistenceOptions::default()).expect("open");
```

### In-memory stubs (unit tests)
Port traits are implemented with local `Mem*` structs backed by `Arc<tokio::sync::Mutex<_>>`:
```rust
struct MemSession(Arc<tokio::sync::Mutex<Option<SessionSnapshot>>>);
struct MemUplinks(Arc<tokio::sync::Mutex<Vec<UplinkRecord>>>);
struct MemAudit(Arc<tokio::sync::Mutex<Vec<String>>>);

#[async_trait]
impl SessionRepository for MemSession {
    async fn get_by_dev_addr(&self, dev_addr: DevAddr) -> AppResult<Option<SessionSnapshot>> {
        let g = self.0.lock().await;
        Ok(g.as_ref().filter(|s| s.dev_addr == dev_addr).cloned())
    }
    async fn upsert(&self, session: &SessionSnapshot) -> AppResult<()> {
        *self.0.lock().await = Some(session.clone());
        Ok(())
    }
}
```

### Stub transport impls (unit + integration tests)
Named behavior stubs for the `RadioTransport` trait:
```rust
struct Hang;          // never completes (tests timeout)
struct AlwaysFail;    // always returns AppError::Infrastructure
struct FailThenSucceed { state: tokio::sync::Mutex<u8> }   // transient failure once
```

### Real UDP socket in integration tests
`radio_transport_resilience.rs` binds a real ephemeral UDP socket to verify the full send/receive path:
```rust
let listener = tokio::net::UdpSocket::bind("127.0.0.1:0").await.expect("bind listener");
let gw = listener.local_addr().expect("listener addr");
let recv = tokio::spawn(async move { ... listener.recv_from(&mut buf).await ... });
```

### Barrier-based concurrency test
`persistence_sqlite.rs` uses `std::sync::Barrier` to orchestrate a deliberate busy-lock contention:
```rust
let barrier = Arc::new(Barrier::new(2));
let t = std::thread::spawn(move || {
    c.execute_batch("BEGIN IMMEDIATE;").expect("begin");
    b2.wait();                       // synchronize with main thread
    std::thread::sleep(Duration::from_millis(150));
    c.execute_batch("COMMIT;").ok();
});
barrier.wait();
let res = UplinkRepository::append(&p, &rec).await;  // must wait on busy lock
t.join().expect("join");
res.expect("append should wait on busy lock");
```

---

## How to Run Tests

```bash
# Run all tests in the workspace
cargo test

# Run only the integration test crate
cargo test -p maverick-integration-tests

# Run a specific integration test file
cargo test -p maverick-integration-tests --test persistence_sqlite

# Run a specific test by name (substring match)
cargo test ingest_happy_path

# Run unit tests in a single crate
cargo test -p maverick-core
cargo test -p maverick-adapter-radio-udp

# Run with output shown (useful for tracing/println in tests)
cargo test -- --nocapture

# Run with single-threaded test executor (useful for timing-sensitive circuit tests)
cargo test -- --test-threads=1
```

---

## What Is Covered

| Area | Coverage level | Notes |
|------|---------------|-------|
| `IngestUplink` use case | Good | Happy path + f_cnt replay rejection |
| `LoRaWAN10xClassA` protocol | Good | Accept, duplicate FCnt, missing session |
| `LnsConfigDocument::validate` | Good | ABP, OTAA, cross-field validation, schema version |
| `StoragePolicy` / `InstallProfile` | Adequate | Numeric assertions on profile fields |
| SQLite port adapter | Good | upsert/read, reopen-recovery, LRU retention, concurrent busy |
| GWMP packet parsing | Good | JSON parse, multi-rxpk, malformed input, binary datagram |
| `ResilientRadioTransport` | Good | Timeout, repeated failure → circuit open, half-open recovery |
| Full operator ingest pipeline (E2E) | Good | `operator_local_gateway_e2e.rs` wires all layers together |
| Extension contracts (serde roundtrip) | Adequate | `SyncBatchEnvelopeV1` JSON roundtrip in `smoke.rs` |

---

## What Is NOT Covered

| Area | Gap | Risk |
|------|-----|------|
| CLI command handlers (`maverick-runtime-edge/src/commands.rs`) | No tests; only manually invoked | Command wiring regressions undetected |
| `ingest/gwmp_loop.rs` (supervised ingest loop) | No automated test; relies on the integration test for `run_radio_ingest_once` indirectly | Loop exit conditions and error paths untested |
| `maverick-extension-tui` beyond config defaults | Only `default_config_has_expected_values` exists | Interactive wizard, profile apply, doctor dashboard not tested |
| `maverick-cloud-core` | Crate exists (`crates/maverick-cloud-core/src/lib.rs`) but no test files found | Unknown |
| Downlink path end-to-end | `DownlinkRepository` / `DownlinkEnqueue` port defined but no use-case test drives it | Downlink persistence logic untested |
| Storage pressure thresholds | Only `db_bytes > 0` asserted; no test for `Elevated`/`Critical`/`HardLimit` transitions | Pressure policy boundary conditions unverified |
| `lns_ops` DB operations (approve/reject/autoprovision) | `sqlite_apply_lns_otaa_without_dev_addr` covers basic apply; approve/reject flows not exercised | Operator workflow regressions possible |

---

## Gaps / Unknowns

- No `nextest` or `tarpaulin` configuration found; code coverage is not measured automatically.
- The `maverick-integration-tests` crate has an empty `src/lib.rs` — all tests are in `tests/` (Cargo integration test convention, not a shared test harness).
- Timing-sensitive tests (`circuit_recovers_after_open_window`, `half_open_trial_closes_after_success`) use short `tokio::time::sleep` durations (30–60 ms). These could be flaky under heavy CI load.
- `maverick-cloud-core` has no tests and its `lib.rs` was not examined in detail; its scope and test status are unknown.
