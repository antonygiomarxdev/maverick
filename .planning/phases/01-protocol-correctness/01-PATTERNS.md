# Phase 1: Protocol Correctness — Pattern Map

**Mapped:** 2026-04-16
**Files analyzed:** 8
**Analogs found:** 8 / 8

---

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `crates/maverick-domain/src/session.rs` | model | transform | Same file (extending existing struct) | exact |
| `crates/maverick-core/src/use_cases/ingest_uplink.rs` | service / use-case | request-response | Same file (extending execute method) | exact |
| `crates/maverick-core/src/protocol/lorawan_10x_class_a.rs` | service / protocol policy | transform | Same file (extending validate_uplink) | exact |
| `crates/maverick-adapter-persistence-sqlite/src/schema.sql` | config / migration | CRUD | `schema.sql` existing tables (sessions, uplinks) | exact |
| `crates/maverick-adapter-persistence-sqlite/src/persistence/lns_ops.rs` | adapter | CRUD | `repos.rs` `run_with_busy_retry` pattern | role-match |
| `crates/maverick-adapter-persistence-sqlite/src/persistence/mod.rs` | adapter | CRUD | `repos.rs` `run_blocking` + `run_with_busy_retry` | exact |
| `crates/maverick-runtime-edge/src/main.rs` | config / composition root | request-response | `commands.rs` `run_setup` exit pattern | role-match |
| `crates/maverick-adapter-radio-udp/src/uplink_ingress.rs` | adapter | request-response | `radio_transport.rs` `UplinkObservation` | role-match |

---

## Pattern Assignments

### `crates/maverick-domain/src/session.rs` (model, transform)

**Analog:** Same file — extend `SessionSnapshot`.

**Current struct** (`session.rs` lines 22–34):
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SessionSnapshot {
    pub dev_eui: DevEui,
    pub dev_addr: DevAddr,
    pub region: RegionId,
    pub class: DeviceClass,
    pub uplink_frame_counter: u32,
    pub downlink_frame_counter: u32,
    #[cfg_attr(feature = "serde", serde(default))]
    pub application_id: Option<String>,
}
```

**Pattern to copy for new fields:** Follow the serde-feature-gated derive pattern. New `nwk_s_key` and `app_s_key` fields are `[u8; 16]` — use the same conditional serde as other fields:
```rust
// Add after application_id:
pub nwk_s_key: [u8; 16],
pub app_s_key: [u8; 16],
```
No `Option` — D-01 says `NOT NULL`. These are session keys, always present once a session exists.

**Key constraint:** `maverick-domain` has no I/O dependencies. The `serde` feature gate (`#[cfg_attr(feature = "serde", ...)]`) must be applied to any derive on these fields, exactly as done for the existing struct-level derive.

---

### `crates/maverick-core/src/use_cases/ingest_uplink.rs` (use-case, request-response)

**Analog:** Same file — extend `execute`.

**Existing execute signature** (lines 18–72):
```rust
pub async fn execute(&self, obs: UplinkObservation) -> AppResult<()> {
    let session = self.sessions.get_by_dev_addr(obs.dev_addr).await?;
    // ... protocol validation ...
    let Some(session) = session else {
        return Err(AppError::NotFound("session".to_string()));
    };
    self.uplinks.append(&UplinkRecord { ... }).await?;
    let mut updated = session;
    updated.uplink_frame_counter = obs.f_cnt;
    self.sessions.upsert(&updated).await?;
    self.audit.emit(AuditRecord { ... }).await?;
    Ok(())
}
```

**Audit pattern for rejections** (lines 28–40) — copy this exact shape for MIC rejection:
```rust
self.audit
    .emit(AuditRecord {
        source: "kernel".to_string(),
        operation: "ingest_uplink".to_string(),
        entity_type: "uplink".to_string(),
        entity_id: Some(format!("{:08x}", obs.dev_addr.0)),
        outcome: format!("rejected:{other:?}"),
        metadata: None,
    })
    .await?;
return Err(AppError::Domain(format!("uplink rejected: {other:?}")));
```

**Error variant pattern for MIC failure** — use `AppError::Domain(String)` matching the existing rejection pattern:
```rust
// MIC rejection — same shape as ProtocolDecision rejection above
return Err(AppError::Domain("mic_invalid".to_string()));
```

**Order of operations in execute (new):**
1. `sessions.get_by_dev_addr` (already present)
2. FCnt 32-bit reconstruction (call `extend_fcnt` from protocol module)
3. Protocol `validate_uplink` (already present, now receives reconstructed u32)
4. MIC verification (new — uses `session.nwk_s_key` + reconstructed fcnt)
5. Dedup check (new — query before persist)
6. `uplinks.append` (already present)
7. `sessions.upsert` (already present)
8. Audit success (already present)

**Test pattern** (lines 76–193) — in-module tests use concrete `MemSession`, `MemUplinks`, `MemAudit` structs with `tokio::sync::Mutex<T>` interior. New tests for MIC and dedup must follow the same pattern:
```rust
#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    // ... same imports ...
    struct MemSession(Arc<tokio::sync::Mutex<Option<SessionSnapshot>>>);
    // impl async_trait::async_trait SessionRepository for MemSession { ... }
    #[tokio::test]
    async fn ingest_rejects_bad_mic() { ... }
}
```

---

### `crates/maverick-core/src/protocol/lorawan_10x_class_a.rs` (protocol policy, transform)

**Analog:** Same file — extend `validate_uplink` and add `extend_fcnt` helper.

**Current validate_uplink** (lines 29–48):
```rust
fn validate_uplink(&self, ctx: ProtocolContext<'_>) -> AppResult<ProtocolDecision> {
    let obs = ctx.observation;
    if !Self::region_supported(obs.region) {
        return Ok(ProtocolDecision::RejectRegionMismatch);
    }
    let Some(session) = ctx.session else {
        return Ok(ProtocolDecision::RejectNoSession);
    };
    if session.class != DeviceClass::ClassA {
        return Ok(ProtocolDecision::RejectUnsupportedClass);
    }
    if session.region != obs.region {
        return Ok(ProtocolDecision::RejectRegionMismatch);
    }
    // LoRaWAN 1.0.x: uplink FCnt must be strictly greater than last seen (32-bit).
    if obs.f_cnt <= session.uplink_frame_counter {
        return Ok(ProtocolDecision::RejectDuplicateFrameCounter);
    }
    Ok(ProtocolDecision::Accept)
}
```

**Helper pattern** — add as a plain `pub fn` on the struct (not in the trait impl), following `region_supported` at lines 10–15:
```rust
impl LoRaWAN10xClassA {
    fn region_supported(region: RegionId) -> bool { ... }

    /// Extend a 16-bit wire FCnt to 32 bits using the session counter.
    /// D-08: `extended = (session_fcnt & 0xFFFF_0000) | wire_u16 as u32`.
    /// Rollover: if extended < session_fcnt and gap > 32768, add 0x1_0000.
    pub fn extend_fcnt(wire_u16: u16, session_fcnt: u32) -> u32 {
        let mut extended = (session_fcnt & 0xFFFF_0000) | u32::from(wire_u16);
        if extended < session_fcnt && session_fcnt - extended > 32768 {
            extended = extended.wrapping_add(0x1_0000);
        }
        extended
    }
}
```

**Test pattern** (lines 52–110) — unit tests in same file, `#[cfg(test)]` module, use `sample_session` / `sample_observation` helpers. Add corresponding tests for `extend_fcnt`:
```rust
#[test]
fn extends_fcnt_no_rollover() {
    assert_eq!(LoRaWAN10xClassA::extend_fcnt(0x0010, 0x0000_0005), 0x0000_0010);
}
#[test]
fn extends_fcnt_rollover() {
    assert_eq!(LoRaWAN10xClassA::extend_fcnt(0x0001, 0x0000_FFFF), 0x0001_0001);
}
```

**`UplinkObservation.f_cnt` type change (D-09):** `f_cnt` lives in `crates/maverick-core/src/ports/radio_transport.rs` line 12. It is currently `u32`. Changing it to `u16` propagates to all call sites. The `validate_uplink` FCnt comparison (`obs.f_cnt <= session.uplink_frame_counter`) must be updated to compare the reconstructed u32 from `extend_fcnt`, not the raw wire value.

---

### `crates/maverick-adapter-persistence-sqlite/src/schema.sql` (DDL, CRUD)

**Analog:** Same file — existing `sessions` table (lines 5–14) and `uplinks` table (lines 16–23).

**Existing sessions DDL** (lines 5–14):
```sql
CREATE TABLE IF NOT EXISTS sessions (
    dev_addr INTEGER PRIMARY KEY NOT NULL,
    dev_eui BLOB NOT NULL,
    region TEXT NOT NULL,
    device_class TEXT NOT NULL,
    uplink_fcnt INTEGER NOT NULL,
    downlink_fcnt INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    application_id TEXT
);
```

**New columns to add** (D-03 — clean break, no migration required):
```sql
CREATE TABLE IF NOT EXISTS sessions (
    dev_addr INTEGER PRIMARY KEY NOT NULL,
    dev_eui BLOB NOT NULL,
    region TEXT NOT NULL,
    device_class TEXT NOT NULL,
    uplink_fcnt INTEGER NOT NULL,
    downlink_fcnt INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    application_id TEXT,
    nwk_s_key BLOB NOT NULL,
    app_s_key BLOB NOT NULL
);
```

**Dedup index on uplinks** (D-10 — add after existing uplinks index at line 23):
```sql
CREATE TABLE IF NOT EXISTS uplinks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    dev_addr INTEGER NOT NULL,
    f_cnt INTEGER NOT NULL,
    payload BLOB NOT NULL,
    received_at_ms INTEGER NOT NULL,
    application_id TEXT
);
CREATE INDEX IF NOT EXISTS idx_uplinks_id ON uplinks(id);
CREATE INDEX IF NOT EXISTS idx_uplinks_dedup ON uplinks(dev_addr, f_cnt, received_at_ms);
```

**Column naming convention:** snake_case matching existing columns (`dev_addr`, `uplink_fcnt`, `updated_at_ms`). BLOB for 16-byte keys (consistent with `dev_eui BLOB`, `apps_key BLOB` in `lns_devices`). `NOT NULL` for keys per D-03.

---

### `crates/maverick-adapter-persistence-sqlite/src/persistence/lns_ops.rs` (adapter, CRUD)

**Analog:** `crates/maverick-adapter-persistence-sqlite/src/persistence/busy.rs` and `repos.rs` — the `run_with_busy_retry` pattern already handles `Mutex::lock` poison via `map_err` (busy.rs lines 29–33):

```rust
let mut guard = self
    .inner
    .conn
    .lock()
    .map_err(|_| AppError::Infrastructure(SQLITE_MUTEX_POISONED.to_string()))?;
```

**Problem in lns_ops.rs** — `.expect()` calls inside `apply_lns_config_inner` (lines 288, 295–296, 312–313, 317, 327, 332, 382, 399–400) bypass `?`-propagation. These are all inside `run_with_busy_retry` closures which return `Result<T, rusqlite::Error>` — so the fix must use `map_err` to convert parse errors into `rusqlite::Error`.

**Pattern for converting parse errors inside rusqlite closures** — copy the `row_to_session` pattern from `sql.rs` lines 99–108:
```rust
let region: RegionId = region_s.parse().map_err(|_| {
    rusqlite::Error::FromSqlConversionFailure(
        0,
        rusqlite::types::Type::Text,
        Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            STORED_FIELD_REGION,
        )),
    )
})?;
```

**Alternative: pre-validate outside the closure** — since `doc.validate()` is called at line 21 before the closure, parse errors inside the closure mean the data was already validated. The cleanest fix is to return a named `rusqlite::Error::InvalidQuery` variant or map to `rusqlite::Error::InvalidParameterName`. Use the existing `map_sqlite` helper from `sql.rs` if returning `AppResult` is possible, or map to `rusqlite::Error` inline as shown above.

**Concrete .expect() locations to fix** (all inside `apply_lns_config_inner`):
- Line 288: `parse_hex_dev_eui(&d.dev_eui).expect("validated lns config")` → `parse_hex_dev_eui(&d.dev_eui).map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?`
- Lines 295–296: `parse_hex_dev_addr(...).expect("validated")` → same pattern
- Lines 312–313, 317: `parse_hex_16/parse_hex_32(...).expect(...)` → same pattern
- Lines 327, 332: optional key parsing `.expect("validated")` → same pattern
- Lines 382, 399–400: ABP device loop, same as above

**`parse_hex_*` must return `Result`** (currently returns `Result` — check if it does; if not, the fix is in `lns_config.rs`). Looking at the call `parse_hex_dev_eui(...).map_err(AppError::InvalidInput)` at line 56 of `lns_ops.rs` — the function already returns `Result`. The `.expect()` calls are just using unwrap instead of `?`.

---

### `crates/maverick-adapter-persistence-sqlite/src/persistence/mod.rs` (adapter, CRUD)

**Analog:** Same file — `run_blocking` pattern (lines 89–97) is the template for any new async method on `SqlitePersistence`:

```rust
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

**New dedup query method pattern** — add to `repos.rs` (as a new `impl SqlitePersistence` method or directly in the `UplinkRepository` impl), following the `run_blocking` + `run_with_busy_retry` two-layer pattern from `repos.rs` lines 72–93:

```rust
// Pattern from repos.rs UplinkRepository::append (lines 72–93):
async fn append(&self, record: &UplinkRecord) -> AppResult<()> {
    let record = record.clone();
    let this = self.clone();
    this.run_blocking(move |p| {                          // outer: tokio blocking thread
        p.run_with_busy_retry(|conn| {                    // inner: SQLite busy retry
            let sql = schema::sql_insert_uplink();
            conn.execute(sql.as_str(), params![...])?;
            p.prune_uplinks_sql(conn)?;
            Ok(())
        })
    })
    .await
}
```

**Dedup query should follow same structure:**
```rust
pub async fn is_duplicate_uplink(
    &self,
    dev_addr: DevAddr,
    f_cnt: u32,
    window_ms: i64,
) -> AppResult<bool> {
    let this = self.clone();
    let key = dev_addr.0 as i64;
    let fcnt = f_cnt as i64;
    this.run_blocking(move |p| {
        p.run_with_busy_retry(|conn| {
            let sql = schema::sql_check_uplink_dedup();   // new schema fn
            let cutoff_ms = now_ms().0 - window_ms;
            let count: i64 = conn.query_row(
                sql.as_str(),
                params![key, fcnt, cutoff_ms],
                |r| r.get(0),
            )?;
            Ok(count > 0)
        })
    })
    .await
}
```

**New schema function** — add to `schema.rs` following the `sql_insert_uplink` pattern (line 79–82):
```rust
pub fn sql_check_uplink_dedup() -> String {
    use names::UPLINKS;
    use uplink_columns::{DEV_ADDR, F_CNT, RECEIVED_AT_MS};
    format!(
        "SELECT COUNT(*) FROM {UPLINKS} WHERE {DEV_ADDR} = ?1 AND {F_CNT} = ?2 AND {RECEIVED_AT_MS} >= ?3"
    )
}
```

---

### `crates/maverick-runtime-edge/src/main.rs` (composition root, request-response)

**Analog:** `crates/maverick-runtime-edge/src/commands.rs` — existing `process::exit` call sites.

**Current pattern** (commands.rs lines 49–107, `run_setup`):
```rust
pub(crate) fn run_setup(non_interactive: bool) {
    // ...
    std::process::exit(2);   // ← direct exit in handler body
    // ...
    std::process::exit(1);
}
```

**Current main.rs dispatch** (lines 182–245): all command handlers are called directly without returning a result — `main` is `async fn main()` with no return type.

**D-18 target pattern** — handlers return `anyhow::Result<()>` (or `AppResult<()>`), `main` maps to exit code. The existing codebase uses `anyhow` in the runtime crate. Pattern:
```rust
// handlers become:
pub(crate) fn run_config_init(config_path: PathBuf, force: bool) -> anyhow::Result<()> {
    // replace: std::process::exit(1)
    // with:    return Err(anyhow::anyhow!("failed to write {}: {e}", path.display()));
    Ok(())
}

// main becomes:
#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    let db_file = EDGE_DB_FILENAME;
    let result: anyhow::Result<()> = match cli.command {
        Commands::Config { cmd } => match cmd {
            ConfigCmd::Init { force, config_path } => config::run_config_init(config_path, force),
            // ...
        },
        // ...
    };
    if let Err(e) = result {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}
```

**`run_setup` exception** — `run_setup` passes through the subprocess exit code (`status.code()`). This is not a "handler error" — it is propagating a child exit code. The clean pattern is to return `i32` or use a dedicated `ExitCode` type:
```rust
// For run_setup specifically, return the intended exit code:
pub(crate) fn run_setup(non_interactive: bool) -> i32 {
    // ...
    if !status.success() {
        return status.code().unwrap_or(1);
    }
    0
}
// In main: std::process::exit(run_setup(non_interactive));
```

**D-19 WAL checkpoint pattern** — `SqlitePersistence` is `Clone` backed by `Arc<Inner>`. The `Drop` impl should trigger `PRAGMA wal_checkpoint(TRUNCATE)`. If no `Drop` exists, add a `close()` method:
```rust
impl SqlitePersistence {
    pub fn close(self) -> AppResult<()> {
        // Drop Arc; if last holder, run checkpoint
        if Arc::strong_count(&self.inner) == 1 {
            let mut conn = self.inner.conn.lock()
                .map_err(|_| AppError::Infrastructure("mutex_poisoned".to_string()))?;
            conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;  // map_sqlite error
        }
        Ok(())
    }
}
```

---

### `crates/maverick-adapter-radio-udp/src/uplink_ingress.rs` (adapter, request-response)

**Analog:** `crates/maverick-core/src/ports/radio_transport.rs` — `UplinkObservation.f_cnt: u32` (line 12).

**Current state:** `uplink_ingress.rs` only contains the `GwmpUdpIngressBackend` marker struct. The actual UDP parsing + `UplinkObservation` construction lives in the ingest loop at `crates/maverick-runtime-edge/src/ingest.rs` (or similar). The `f_cnt` type change (D-09) flows from `UplinkObservation` (core port) to wherever the GWMP `rxpk` byte is read.

**Current `UplinkObservation`** (`radio_transport.rs` line 12):
```rust
pub struct UplinkObservation {
    // ...
    pub f_cnt: u32,   // ← currently u32; D-09 changes to u16 (wire value)
    // ...
}
```

**Pattern for the type change** — change `f_cnt: u32` to `f_cnt: u16` in `UplinkObservation`. Every construction site must provide a `u16`. The GWMP parser reads a 2-byte little-endian field from the PHYPayload `FHDR.FCnt` — it is natively `u16`. No cast needed at parse time; the `as u16` cast is eliminated.

**Call site pattern** (from `ingest_uplink.rs` test helper lines 120–131):
```rust
fn obs(fc: u32) -> UplinkObservation {
    UplinkObservation {
        // ...
        f_cnt: fc,   // ← becomes fc as u16 after type change
        // ...
    }
}
```

**After type change**, `extend_fcnt(obs.f_cnt, session.uplink_frame_counter)` in `execute` receives `u16` directly — no cast needed as `extend_fcnt` takes `wire_u16: u16`.

---

## Shared Patterns

### `run_with_busy_retry` — Mutex Lock + SQLite Error Propagation
**Source:** `crates/maverick-adapter-persistence-sqlite/src/persistence/busy.rs` lines 23–48
**Apply to:** All new or modified SQLite operations in `lns_ops.rs`, `mod.rs`, `repos.rs`
```rust
// Mutex poison is already handled by run_with_busy_retry:
let mut guard = self
    .inner
    .conn
    .lock()
    .map_err(|_| AppError::Infrastructure(SQLITE_MUTEX_POISONED.to_string()))?;
// DO NOT call .expect() or .unwrap() inside any closure passed to run_with_busy_retry.
// All errors must propagate with ?
```

### `run_blocking` — Async Wrapper for SQLite
**Source:** `crates/maverick-adapter-persistence-sqlite/src/persistence/mod.rs` lines 89–97
**Apply to:** Any new `async fn` on `SqlitePersistence` (dedup check, `close()`)
```rust
let this = self.clone();
this.run_blocking(move |p| {
    p.run_with_busy_retry(|conn| {
        // synchronous SQLite work here
        Ok(())
    })
})
.await
```

### Audit Emit Pattern
**Source:** `crates/maverick-core/src/use_cases/ingest_uplink.rs` lines 28–39
**Apply to:** MIC rejection audit, dedup-discard (no audit per D-10), any new rejection path in `execute`
```rust
self.audit.emit(AuditRecord {
    source: "kernel".to_string(),
    operation: "ingest_uplink".to_string(),
    entity_type: "uplink".to_string(),
    entity_id: Some(format!("{:08x}", obs.dev_addr.0)),
    outcome: "rejected:mic_invalid".to_string(),
    metadata: None,
}).await?;
```

### AppError Variants
**Source:** `crates/maverick-core/src/error.rs`
**Apply to:** All new error paths
```
AppError::Domain(String)       — MIC failure, FCnt rejection (protocol rule violation)
AppError::Infrastructure(String) — Mutex poison, SQLite I/O, task join failure
AppError::NotFound(String)     — session missing after ProtocolDecision::Accept
AppError::InvalidInput(String) — hex parse failures in config sync (lns_ops.rs)
```

### Schema Column Constants
**Source:** `crates/maverick-adapter-persistence-sqlite/src/schema.rs` `sessions_columns` and `uplink_columns` modules
**Apply to:** All new SQL query builder functions
```rust
// Follow existing pattern — add new column name constants before using them in sql_* fns:
pub mod sessions_columns {
    // existing...
    pub const NWK_S_KEY: &str = "nwk_s_key";
    pub const APP_S_KEY: &str = "app_s_key";
}
pub mod uplink_columns {
    // existing...
    pub const RECEIVED_AT_MS: &str = "received_at_ms";
}
```

### `row_to_session` — Row Mapping Error Convention
**Source:** `crates/maverick-adapter-persistence-sqlite/src/persistence/sql.rs` lines 87–128
**Apply to:** Updated `row_to_session` must read the new `nwk_s_key` and `app_s_key` BLOB columns (indices 7 and 8 after `application_id` at index 6)
```rust
// Existing pattern for BLOB → fixed array:
let mut eui_arr = [0u8; DEV_EUI_BYTE_LEN];
if dev_eui_bytes.len() == DEV_EUI_BYTE_LEN {
    eui_arr.copy_from_slice(&dev_eui_bytes[..DEV_EUI_BYTE_LEN]);
}
// New key fields follow same shape:
let nwk_s_key_bytes: Vec<u8> = row.get(7)?;
let mut nwk_s_key = [0u8; 16];
if nwk_s_key_bytes.len() == 16 {
    nwk_s_key.copy_from_slice(&nwk_s_key_bytes);
}
```

### serde Feature Gate (domain crate)
**Source:** `crates/maverick-domain/src/session.rs` lines 6, 22–23
**Apply to:** Any new fields on `SessionSnapshot` or other domain structs
```rust
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
// For individual fields that need special serde handling:
#[cfg_attr(feature = "serde", serde(default))]
```

---

## No Analog Found

All 8 files have existing analogs in the codebase. There are no net-new file roles.

The only net-new logic is:
- AES-128 CMAC (MIC computation) — no existing crypto in codebase; follow `aes 0.8.x` + `cmac 0.7.x` from RESEARCH.md
- AES-128 CTR (FRMPayload decryption) — same, no existing analog; follow LoRaWAN 1.0.3 §4.3.2

For both crypto primitives, the integration point is `IngestUplink::execute` which already has the analog pattern for how to call stateless helpers and propagate errors.

---

## Metadata

**Analog search scope:** All crates in `crates/` workspace
**Files scanned:** 14 source files read directly
**Pattern extraction date:** 2026-04-16
