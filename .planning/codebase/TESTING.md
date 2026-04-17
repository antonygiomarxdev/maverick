# Testing Patterns

**Analysis Date:** 2026-04-16

## Test Framework

**Test Runner:** Rust's built-in `#[test]` with `tokio::test` for async tests

**Assertion Library:** Standard `assert!`, `assert_eq!`, `assert!(matches!(...))` macros

**Key Dependencies:**
- `tokio` with `macros` and `rt-multi-thread` features
- `async-trait` for async trait methods in test adapters
- `tempfile` for temporary test directories
- `rusqlite` for integration tests requiring SQLite

**Run Commands:**
```bash
cargo test --workspace                              # Run all tests
cargo test -p <name>                               # Run tests for specific crate
cargo test -p maverick-integration-tests --test <name>  # Run specific integration test
cargo test -- --nocapture                          # Show output (tracing/println)
cargo test -- --test-threads=1                     # Single-threaded for timing-sensitive tests
```

## Test File Organization

**Location:**
- Unit tests: `#[cfg(test)] mod tests` at bottom of each source file
- Integration tests: `crates/maverick-integration-tests/tests/*.rs`

**Integration Test Structure:**
```
crates/maverick-integration-tests/
├── src/lib.rs              # Test helpers (currently empty)
└── tests/
    ├── smoke.rs            # Quick smoke tests
    ├── operator_local_gateway_e2e.rs
    ├── persistence_sqlite.rs
    └── radio_transport_resilience.rs
```

**Unit Test Location Pattern:**
```rust
// Bottom of crates/maverick-core/src/use_cases/ingest_uplink.rs
#[cfg(test)]
mod tests {
    use super::*;
    // in-memory stub implementations + #[tokio::test] cases
}
```

## Test Types

| Type | Location | Scope |
|------|----------|-------|
| Unit tests | `#[cfg(test)] mod tests` at bottom of each source file | Single module / function in isolation |
| Integration tests | `crates/maverick-integration-tests/tests/*.rs` | Multi-crate composition against real adapters |

## Test Structure Patterns

**Sync Unit Test:**
```rust
#[test]
fn sqlite_ddl_defines_tables_matching_schema_names() {
    // Arrange
    let dir = tempfile::tempdir().expect("tempdir");
    let db = dir.path().join("test.db");

    // Act
    let persistence = SqlitePersistence::open(...).expect("open");

    // Assert
    assert!(/* condition */);
}
```

**Async Integration Test:**
```rust
#[tokio::test]
async fn operator_local_gateway_flow_ingests_and_persists_uplink() {
    // 1. Setup persistence
    let store = SqlitePersistence::open(&db, policy, options).expect("open");

    // 2. Setup session
    let session = SessionSnapshot { ... };
    SessionRepository::upsert(&store, &session).await.expect("upsert");

    // 3. Execute use case
    svc.execute(obs).await.expect("ingest");

    // 4. Verify persistence
    let persisted = SessionRepository::get_by_dev_addr(&store, dev_addr).await.expect("get");
    assert_eq!(persisted.uplink_frame_counter, expected);
}
```

## Mocking Patterns

**In-Memory Stubs (Unit Tests):**
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

**Stub Transport Impls:**
```rust
struct UdpRadioStub;

#[async_trait::async_trait]
impl RadioTransport for UdpRadioStub {
    async fn send_downlink(&self, _frame: &DownlinkFrame) -> AppResult<()> {
        Err(AppError::Infrastructure("stub".to_string()))
    }
}
```

**Custom Test Doubles:**
```rust
struct FailThenSucceed {
    state: tokio::sync::Mutex<u8>,
}

#[async_trait::async_trait]
impl RadioTransport for FailThenSucceed {
    async fn send_downlink(&self, _frame: &DownlinkFrame) -> AppResult<()> {
        let mut g = self.state.lock().await;
        if *g < 1 {
            *g += 1;
            Err(AppError::Infrastructure("transient".to_string()))
        } else {
            Ok(())
        }
    }
}
```

**What to Mock:**
- Transport adapters (stub for unit tests)
- Repository implementations (use concrete SQLite for integration)

**What NOT to Mock:**
- Domain types (use real value objects)
- Simple serialization/deserialization (test with real types)

## Fixtures and Factories

**Test Data Construction:**
- Inline construction for domain objects: `DevEui(Eui64([0x11; 8]))`
- `SessionSnapshot` literals with all fields explicit
- Sample JSON as string literals

**File Fixtures:**
- `tempfile::tempdir()` for ephemeral database files
- Temporary directories cleaned up automatically

**Example JSON Fixture:**
```rust
let gwmp_json = r#"{
  "rxpk":[
    {"freq":868.1,"rssi":-57,"lsnr":5.2,"data":"QAECAwQEAAEByv66vg=="}
  ]
}"#;
```

## Common Test Patterns

**Error Testing:**
```rust
#[tokio::test]
async fn parse_failure_is_reported_without_panic() {
    let malformed = b"not-gwmp";
    let err = parse_push_data(malformed).expect_err("expected parse failure");
    assert!(matches!(err, AppError::InvalidInput(_)));
}
```

**Circuit Breaker Testing:**
```rust
#[tokio::test]
async fn circuit_recovers_after_open_window_and_successful_trial() {
    let policy = ResiliencePolicy {
        max_retries: 0,
        circuit_failure_threshold: 1,
        circuit_open_duration: std::time::Duration::from_millis(30),
        ..ResiliencePolicy::default()
    };
    // ... test body
    tokio::time::sleep(std::time::Duration::from_millis(40)).await;
    let _ = transport.send_downlink(&frame).await;
    assert_eq!(transport.circuit_state(), CircuitStateView::Closed);
}
```

**Real UDP Socket Testing:**
```rust
let listener = tokio::net::UdpSocket::bind("127.0.0.1:0")
    .await
    .expect("bind listener");
let gw = listener.local_addr().expect("listener addr");
let recv = tokio::spawn(async move {
    let mut buf = [0_u8; 64];
    let (n, _) = listener.recv_from(&mut buf).await.expect("recv");
    buf[..n].to_vec()
});
```

**Concurrency Testing (Busy Lock):**
```rust
let barrier = Arc::new(Barrier::new(2));
let t = std::thread::spawn(move || {
    c.execute_batch("BEGIN IMMEDIATE;").expect("begin");
    b2.wait();
    std::thread::sleep(Duration::from_millis(150));
    c.execute_batch("COMMIT;").ok();
});
barrier.wait();
let res = UplinkRepository::append(&p, &rec).await;
res.expect("append should wait on busy lock");
```

## CI Setup

**Workflow:** `.github/workflows/ci.yml`

**Test Job:**
```yaml
test:
  name: Test
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - name: Setup Rust
      uses: dtolnay/rust-toolchain@stable
    - name: Cache cargo
      uses: Swatinem/rust-cache@v2
    - name: Tests
      run: cargo test --workspace
```

**Additional CI Jobs:**
- `lint`: Format check + Clippy (`-D warnings`)
- `audit`: Security audit with `cargo audit`

## Coverage

**No enforced coverage target.** Quality over percentage.

**Covered Areas:**
| Area | Level |
|------|-------|
| `IngestUplink` use case | Good - happy path + f_cnt replay rejection |
| `LoRaWAN10xClassA` protocol | Good - accept, duplicate FCnt, missing session |
| `LnsConfigDocument::validate` | Good - ABP, OTAA, cross-field, schema version |
| SQLite port adapter | Good - upsert/read, reopen-recovery, LRU retention, concurrent busy |
| GWMP packet parsing | Good - JSON parse, multi-rxpk, malformed input, binary |
| `ResilientRadioTransport` | Good - timeout, circuit open, half-open recovery |
| Full operator ingest pipeline | Good - E2E wires all layers |
| Extension contracts serde | Adequate - `SyncBatchEnvelopeV1` JSON roundtrip |

**Gaps:**
| Area | Gap |
|------|-----|
| CLI command handlers | No tests |
| `ingest/gwmp_loop.rs` | No automated test |
| `maverick-extension-tui` | Only config defaults tested |
| `maverick-cloud-core` | No tests found |
| Downlink path end-to-end | No use-case test |
| Storage pressure thresholds | Only `db_bytes > 0` asserted |
| `lns_ops` approve/reject | Not exercised |

## Test Naming

**Pattern:** `feature_description` or `scenario_behavior`

**Examples:**
- `region_parse_roundtrip` - parse then serialize back
- `operator_local_gateway_flow_ingests_and_persists_uplink` - E2E scenario
- `circuit_recovers_after_open_window_and_successful_trial` - resilience pattern
- `stub_adapter_fails_without_panicking_kernel_contract` - contract verification

---

*Testing analysis: 2026-04-16*
