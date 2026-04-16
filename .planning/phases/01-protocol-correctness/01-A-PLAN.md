---
phase: 01-protocol-correctness
plan: A
type: execute
wave: 1
depends_on: []
files_modified:
  - crates/maverick-domain/src/session.rs
  - crates/maverick-core/src/ports/radio_transport.rs
  - crates/maverick-core/src/ports/uplink_repository.rs
  - crates/maverick-adapter-persistence-sqlite/src/schema.sql
  - crates/maverick-adapter-persistence-sqlite/src/schema.rs
  - crates/maverick-adapter-persistence-sqlite/src/persistence/sql.rs
autonomous: true
requirements:
  - PROT-03
  - PROT-04
  - CORE-02

must_haves:
  truths:
    - "SessionSnapshot carries nwk_s_key and app_s_key as [u8;16] fields"
    - "UplinkObservation carries wire_mic:[u8;4] and phy_without_mic:Vec<u8> fields, and f_cnt is u16"
    - "UplinkRecord carries payload_decrypted:Option<Vec<u8>>"
    - "sessions SQLite table has nwk_s_key BLOB and app_s_key BLOB columns"
    - "uplinks SQLite table has received_at_ms INTEGER column and dedup index"
    - "row_to_session reads nwk_s_key and app_s_key at column indices 7 and 8"
  artifacts:
    - path: "crates/maverick-domain/src/session.rs"
      provides: "SessionSnapshot with key fields"
      contains: "nwk_s_key: [u8; 16]"
    - path: "crates/maverick-core/src/ports/radio_transport.rs"
      provides: "UplinkObservation with wire_mic, phy_without_mic, f_cnt:u16"
      contains: "wire_mic: [u8; 4]"
    - path: "crates/maverick-core/src/ports/uplink_repository.rs"
      provides: "UplinkRecord with payload_decrypted"
      contains: "payload_decrypted: Option<Vec<u8>>"
    - path: "crates/maverick-adapter-persistence-sqlite/src/schema.sql"
      provides: "DDL with new columns and dedup index"
      contains: "nwk_s_key BLOB NOT NULL"
    - path: "crates/maverick-adapter-persistence-sqlite/src/schema.rs"
      provides: "Column constants and SQL builder functions for new columns"
      contains: "NWK_S_KEY"
    - path: "crates/maverick-adapter-persistence-sqlite/src/persistence/sql.rs"
      provides: "row_to_session updated to read key columns"
      contains: "nwk_s_key_bytes"
  key_links:
    - from: "crates/maverick-adapter-persistence-sqlite/src/persistence/sql.rs"
      to: "crates/maverick-adapter-persistence-sqlite/src/schema.sql"
      via: "DDL_INIT include_str! — column order must match row_to_session index offsets"
      pattern: "row\\.get\\(7\\)"
    - from: "crates/maverick-adapter-persistence-sqlite/src/persistence/repos.rs"
      to: "crates/maverick-core/src/ports/uplink_repository.rs"
      via: "UplinkRecord.payload_decrypted used in append INSERT"
      pattern: "payload_decrypted"
---

<objective>
Establish the domain model and schema foundations that all other Phase 1 plans depend on.

Purpose: MIC verification (Plan C) needs NwkSKey on SessionSnapshot. Payload decryption (Plan C) needs AppSKey. The dedup query (Plan D) needs received_at_ms on uplinks. The f_cnt:u16 type change (Plan B) is a breaking change to UplinkObservation that must be stable before any other plan uses it. This plan makes all of those changes atomically so downstream plans have a coherent base.

Output: Updated domain types, updated port types, updated schema DDL, updated schema constants, updated row mapper. The workspace will not compile until Plan B and Plan C update their call sites.
</objective>

<execution_context>
@/root/.claude/get-shit-done/workflows/execute-plan.md
@/root/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/phases/01-protocol-correctness/1-CONTEXT.md
@.planning/phases/01-protocol-correctness/01-RESEARCH.md
@.planning/phases/01-protocol-correctness/01-PATTERNS.md
</context>

<tasks>

<task type="auto">
  <name>Task A-1: Extend domain model and port types</name>
  <files>
    crates/maverick-domain/src/session.rs
    crates/maverick-core/src/ports/radio_transport.rs
    crates/maverick-core/src/ports/uplink_repository.rs
  </files>
  <read_first>
    - crates/maverick-domain/src/session.rs
    - crates/maverick-core/src/ports/radio_transport.rs
    - crates/maverick-core/src/ports/uplink_repository.rs
    - crates/maverick-core/src/use_cases/ingest_uplink.rs
  </read_first>
  <action>
**session.rs** — Add two fields to `SessionSnapshot` after `application_id`. Follow the same `#[cfg_attr(feature = "serde", serde(default))]` pattern ONLY if needed for backwards compat; these fields are NOT optional (per D-01: `NOT NULL`), so add them without `serde(default)`. The struct now reads:
```rust
pub nwk_s_key: [u8; 16],
pub app_s_key: [u8; 16],
```
Add them after `application_id: Option<String>`. The serde feature gate `#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]` is already on the struct — no additional attribute needed for these fields since `[u8; 16]` serde-serializes by default.

NOTE: This breaks the `SessionSnapshot { ... }` struct literal in `ingest_uplink.rs` tests. That is expected and will be fixed in Plan C. The workspace will not fully compile after this task until Plan C is complete.

**radio_transport.rs** — Make three changes to `UplinkObservation`:
1. Change `pub f_cnt: u32` to `pub f_cnt: u16` (per D-09 — wire value is always 16-bit; reconstruction to u32 happens in protocol module)
2. Add `pub wire_mic: [u8; 4]` field (per locked decision — MIC bytes preserved from parser)
3. Add `pub phy_without_mic: Vec<u8>` field (the raw PHY payload excluding the last 4 MIC bytes; needed by MIC verifier in Plan C)

Place `wire_mic` and `phy_without_mic` after the existing `snr` field.

NOTE: This breaks `ingest_uplink.rs` test helper `obs(fc: u32)` and `lorawan_10x_class_a.rs` test helper `sample_observation`. These are fixed in Plans B and C respectively.

**uplink_repository.rs** — Add `payload_decrypted: Option<Vec<u8>>` field to `UplinkRecord` after `application_id`. Also add `received_at_ms: i64` field (milliseconds since epoch; required by dedup query in Plan D) after `f_cnt`. The struct now has 5 fields:
```rust
pub struct UplinkRecord {
    pub dev_addr: DevAddr,
    pub f_cnt: u32,             // reconstructed 32-bit value (set by IngestUplink)
    pub received_at_ms: i64,
    pub payload: Vec<u8>,
    pub application_id: Option<String>,
    pub payload_decrypted: Option<Vec<u8>>,
}
```

NOTE: `UplinkRecord` is constructed in `ingest_uplink.rs` — that call site breaks and is fixed in Plan C.
  </action>
  <verify>
    <automated>cargo check -p maverick-domain 2>&1 | grep -E "^error" | head -5</automated>
  </verify>
  <done>
    - `SessionSnapshot` compiles with `nwk_s_key: [u8; 16]` and `app_s_key: [u8; 16]` fields
    - `UplinkObservation.f_cnt` is `u16`, `wire_mic: [u8; 4]` and `phy_without_mic: Vec<u8>` fields exist
    - `UplinkRecord` has `payload_decrypted: Option<Vec<u8>>` and `received_at_ms: i64` fields
    - `cargo check -p maverick-domain` passes with no errors
    - `cargo check -p maverick-core` will have errors (expected — call sites not yet updated)
  </done>
</task>

<task type="auto">
  <name>Task A-2: Schema DDL, column constants, and row_to_session update</name>
  <files>
    crates/maverick-adapter-persistence-sqlite/src/schema.sql
    crates/maverick-adapter-persistence-sqlite/src/schema.rs
    crates/maverick-adapter-persistence-sqlite/src/persistence/sql.rs
  </files>
  <read_first>
    - crates/maverick-adapter-persistence-sqlite/src/schema.sql
    - crates/maverick-adapter-persistence-sqlite/src/schema.rs
    - crates/maverick-adapter-persistence-sqlite/src/persistence/sql.rs
  </read_first>
  <action>
**schema.sql** — Make the following DDL changes:

1. `sessions` table: Add `nwk_s_key BLOB NOT NULL` and `app_s_key BLOB NOT NULL` after `application_id TEXT`. The `CREATE TABLE IF NOT EXISTS sessions` block now ends with these two columns. Since there are no production users (per D-03 / CONTEXT.md), this is a clean break — no migration needed for the initial DDL change. The `init_schema` migration functions in `sql.rs` will handle `ALTER TABLE` for any existing dev databases.

2. `uplinks` table: Add `received_at_ms INTEGER NOT NULL` after `payload BLOB NOT NULL`. Also add `payload_decrypted BLOB` (nullable) after `received_at_ms`.

3. Add dedup index after the existing `idx_uplinks_id` index:
```sql
CREATE INDEX IF NOT EXISTS idx_uplinks_dedup ON uplinks(dev_addr, f_cnt, received_at_ms);
```

**schema.rs** — Add new column name constants:

In `sessions_columns` module, add:
```rust
pub const NWK_S_KEY: &str = "nwk_s_key";
pub const APP_S_KEY: &str = "app_s_key";
```

In `uplink_columns` module, add:
```rust
pub const RECEIVED_AT_MS: &str = "received_at_ms";
pub const PAYLOAD_DECRYPTED: &str = "payload_decrypted";
```

Add a new SQL builder function for the dedup query:
```rust
pub fn sql_check_uplink_dedup() -> String {
    use names::UPLINKS;
    use uplink_columns::{DEV_ADDR, F_CNT, RECEIVED_AT_MS};
    format!(
        "SELECT COUNT(*) FROM {UPLINKS} WHERE {DEV_ADDR} = ?1 AND {F_CNT} = ?2 AND {RECEIVED_AT_MS} >= ?3"
    )
}
```

Update `sql_select_session_by_dev_addr()` to include `nwk_s_key` and `app_s_key` in the SELECT list:
```rust
pub fn sql_select_session_by_dev_addr() -> String {
    use names::SESSIONS;
    use sessions_columns::{
        APPLICATION_ID, APP_S_KEY, DEVICE_CLASS, DEV_ADDR, DEV_EUI, DOWNLINK_FCNT,
        NWK_S_KEY, REGION, UPLINK_FCNT,
    };
    format!(
        "SELECT {DEV_ADDR}, {DEV_EUI}, {REGION}, {DEVICE_CLASS}, {UPLINK_FCNT}, {DOWNLINK_FCNT}, \
         {APPLICATION_ID}, {NWK_S_KEY}, {APP_S_KEY} \
         FROM {SESSIONS} WHERE {DEV_ADDR} = ?1"
    )
}
```

Update `sql_upsert_session()` to INSERT and UPDATE `nwk_s_key` (param ?9) and `app_s_key` (param ?10) — add them to both the column list and ON CONFLICT UPDATE list.

Update `sql_insert_uplink()` to include `received_at_ms` (?5) and `payload_decrypted` (?6):
```rust
pub fn sql_insert_uplink() -> String {
    use names::UPLINKS;
    use uplink_columns::{APPLICATION_ID, DEV_ADDR, F_CNT, PAYLOAD, PAYLOAD_DECRYPTED, RECEIVED_AT_MS};
    format!(
        "INSERT INTO {UPLINKS} ({DEV_ADDR}, {F_CNT}, {RECEIVED_AT_MS}, {PAYLOAD}, {APPLICATION_ID}, {PAYLOAD_DECRYPTED}) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)"
    )
}
```

**sql.rs** — Three changes:

1. Add migration functions to `init_schema` for existing dev databases. Call them from `init_schema` after the existing migrations:
```rust
pub(crate) fn init_schema(conn: &mut Connection) -> Result<(), AppError> {
    conn.execute_batch(schema::DDL_INIT)
        .map_err(|e| map_sqlite(SqliteOperation::Schema, e))?;
    migrate_legacy_columns(conn)?;
    migrate_lns_devices_v2(conn)?;
    migrate_sessions_v2(conn)?;  // ADD
    migrate_uplinks_v2(conn)?;   // ADD
    Ok(())
}

/// Add nwk_s_key and app_s_key columns to sessions (for DBs created before Phase 1).
fn migrate_sessions_v2(conn: &mut Connection) -> Result<(), AppError> {
    let _ = conn.execute("ALTER TABLE sessions ADD COLUMN nwk_s_key BLOB", []);
    let _ = conn.execute("ALTER TABLE sessions ADD COLUMN app_s_key BLOB", []);
    Ok(())
}

/// Add received_at_ms and payload_decrypted to uplinks; add dedup index.
fn migrate_uplinks_v2(conn: &mut Connection) -> Result<(), AppError> {
    let _ = conn.execute("ALTER TABLE uplinks ADD COLUMN received_at_ms INTEGER", []);
    let _ = conn.execute("ALTER TABLE uplinks ADD COLUMN payload_decrypted BLOB", []);
    let _ = conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_uplinks_dedup ON uplinks(dev_addr, f_cnt, received_at_ms)",
        [],
    );
    Ok(())
}
```

2. Update `row_to_session` to read `nwk_s_key` (index 7) and `app_s_key` (index 8) from the result row. Follow the existing `eui_arr` BLOB-to-array pattern exactly:
```rust
let nwk_s_key_bytes: Vec<u8> = row.get(7)?;
let mut nwk_s_key = [0u8; 16];
if nwk_s_key_bytes.len() == 16 {
    nwk_s_key.copy_from_slice(&nwk_s_key_bytes);
}
let app_s_key_bytes: Vec<u8> = row.get(8)?;
let mut app_s_key = [0u8; 16];
if app_s_key_bytes.len() == 16 {
    app_s_key.copy_from_slice(&app_s_key_bytes);
}
```
Add `nwk_s_key` and `app_s_key` to the `SessionSnapshot { ... }` constructor at the end of `row_to_session`.

3. Update `repos.rs` — the `upsert` method constructs params for `sql_upsert_session`. It must now pass `session.nwk_s_key` (as `&session.nwk_s_key[..]`) and `session.app_s_key` (as `&session.app_s_key[..]`) as params ?9 and ?10.

Also update `repos.rs` — the `append` method in `UplinkRepository` must pass `record.received_at_ms` (?3), `&record.payload` (?4), `record.application_id` (?5), and `record.payload_decrypted.as_deref()` (?6) to match the new `sql_insert_uplink()` signature.

IMPORTANT: `repos.rs` is in `persistence/repos.rs`. Update the params! macro calls there.
  </action>
  <verify>
    <automated>cargo check -p maverick-adapter-persistence-sqlite 2>&1 | grep -E "^error" | head -10</automated>
  </verify>
  <done>
    - `schema.sql` sessions table has `nwk_s_key BLOB NOT NULL` and `app_s_key BLOB NOT NULL`
    - `schema.sql` uplinks table has `received_at_ms INTEGER NOT NULL`, `payload_decrypted BLOB`, and `idx_uplinks_dedup` index
    - `schema.rs` has `NWK_S_KEY`, `APP_S_KEY`, `RECEIVED_AT_MS`, `PAYLOAD_DECRYPTED` constants
    - `schema.rs` has `sql_check_uplink_dedup()` function
    - `sql.rs` `init_schema` calls `migrate_sessions_v2` and `migrate_uplinks_v2`
    - `sql.rs` `row_to_session` reads key columns at indices 7 and 8
    - `cargo check -p maverick-adapter-persistence-sqlite` passes
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| schema DDL → SQLite engine | DDL changes applied at startup via `execute_batch`; malformed SQL would crash open |
| row_to_session | Maps raw SQLite row bytes to domain types; index mismatch is a silent data corruption |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-01-A-01 | Information Disclosure | SessionSnapshot nwk_s_key / app_s_key | accept | Keys stored as BLOB (not logged); SEC-02 (encryption at rest) deferred to Phase 4 per REQUIREMENTS.md; add schema comment documenting plaintext status |
| T-01-A-02 | Tampering | row_to_session index offsets | mitigate | Use named column indices derived from SELECT column order in `sql_select_session_by_dev_addr`; if the SELECT changes, the indices must be updated in sync |
| T-01-A-03 | Denial of Service | ALTER TABLE migration silently fails | accept | `let _ = conn.execute(...)` ignores errors intentionally; migration is best-effort for dev DBs; clean installs use the updated DDL_INIT |
</threat_model>

<verification>
After both tasks complete:

```bash
cargo check -p maverick-domain
cargo check -p maverick-adapter-persistence-sqlite
grep -n "nwk_s_key" crates/maverick-domain/src/session.rs
grep -n "wire_mic" crates/maverick-core/src/ports/radio_transport.rs
grep -n "payload_decrypted" crates/maverick-core/src/ports/uplink_repository.rs
grep -n "nwk_s_key BLOB" crates/maverick-adapter-persistence-sqlite/src/schema.sql
grep -n "received_at_ms" crates/maverick-adapter-persistence-sqlite/src/schema.sql
grep -n "idx_uplinks_dedup" crates/maverick-adapter-persistence-sqlite/src/schema.sql
grep -n "sql_check_uplink_dedup" crates/maverick-adapter-persistence-sqlite/src/schema.rs
grep -n "migrate_sessions_v2\|migrate_uplinks_v2" crates/maverick-adapter-persistence-sqlite/src/persistence/sql.rs
```

NOTE: `cargo test --workspace` will fail at this point due to call-site breakage in Plans B and C. That is expected. This plan's verification only checks the domain/adapter crates.
</verification>

<success_criteria>
- `SessionSnapshot` has `nwk_s_key: [u8; 16]` and `app_s_key: [u8; 16]` — grep-verifiable
- `UplinkObservation.f_cnt` is `u16` and carries `wire_mic: [u8; 4]` and `phy_without_mic: Vec<u8>` — grep-verifiable
- `UplinkRecord` has `payload_decrypted: Option<Vec<u8>>` and `received_at_ms: i64` — grep-verifiable
- `schema.sql` sessions table has `nwk_s_key BLOB NOT NULL` and `app_s_key BLOB NOT NULL` — grep-verifiable
- `schema.sql` uplinks table has `received_at_ms INTEGER NOT NULL` and `idx_uplinks_dedup` index — grep-verifiable
- `sql.rs` `row_to_session` reads key bytes at indices 7 and 8 — grep-verifiable
- `cargo check -p maverick-domain` and `cargo check -p maverick-adapter-persistence-sqlite` pass
</success_criteria>

<output>
After completion, create `.planning/phases/01-protocol-correctness/01-A-SUMMARY.md`
</output>
