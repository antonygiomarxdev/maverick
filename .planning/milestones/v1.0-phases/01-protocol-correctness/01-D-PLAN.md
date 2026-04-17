---
phase: 01-protocol-correctness
plan: D
type: execute
wave: 4
depends_on:
  - 01-A
  - 01-C
files_modified:
  - crates/maverick-core/src/ports/uplink_repository.rs
  - crates/maverick-adapter-persistence-sqlite/src/persistence/repos.rs
  - crates/maverick-core/src/use_cases/ingest_uplink.rs
autonomous: true
requirements:
  - PROT-06

must_haves:
  truths:
    - "UplinkRepository trait has is_duplicate method"
    - "SqlitePersistence implements is_duplicate with a SQLite COUNT query against the dedup index"
    - "IngestUplink::execute calls is_duplicate before uplinks.append and returns Ok(()) silently on dup"
    - "Duplicate frames (same dev_addr + reconstructed_fcnt within 30s) are discarded without audit noise"
  artifacts:
    - path: "crates/maverick-core/src/ports/uplink_repository.rs"
      provides: "UplinkRepository trait with is_duplicate method"
      contains: "is_duplicate"
    - path: "crates/maverick-adapter-persistence-sqlite/src/persistence/repos.rs"
      provides: "SqlitePersistence impl of is_duplicate using sql_check_uplink_dedup"
      contains: "is_duplicate"
    - path: "crates/maverick-core/src/use_cases/ingest_uplink.rs"
      provides: "Dedup check before append in execute"
      contains: "is_duplicate"
  key_links:
    - from: "crates/maverick-core/src/use_cases/ingest_uplink.rs"
      to: "crates/maverick-core/src/ports/uplink_repository.rs"
      via: "self.uplinks.is_duplicate(dev_addr, reconstructed_fcnt, 30_000) called before append"
      pattern: "is_duplicate"
    - from: "crates/maverick-adapter-persistence-sqlite/src/persistence/repos.rs"
      to: "crates/maverick-adapter-persistence-sqlite/src/schema.rs"
      via: "sql_check_uplink_dedup() query uses idx_uplinks_dedup index"
      pattern: "sql_check_uplink_dedup"
---

<objective>
Add SQLite-backed duplicate frame detection to the ingest pipeline.

Purpose: PROT-06 requires that a duplicate uplink frame (same DevAddr + FCnt arriving within 30 seconds, e.g. from two gateways hearing the same transmission) is discarded silently — only one copy persisted. The dedup query is SQLite-backed (D-10/D-11) so it survives process restarts.

Output: `is_duplicate` method on `UplinkRepository` trait, `SqlitePersistence` implementation, and integration in `execute`.
</objective>

<execution_context>
@/root/.claude/get-shit-done/workflows/execute-plan.md
@/root/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/phases/01-protocol-correctness/1-CONTEXT.md
@.planning/phases/01-protocol-correctness/01-RESEARCH.md
@.planning/phases/01-protocol-correctness/01-PATTERNS.md
@.planning/phases/01-protocol-correctness/01-A-SUMMARY.md
@.planning/phases/01-protocol-correctness/01-C-SUMMARY.md
</context>

<tasks>

<task type="auto">
  <name>Task D-1: Add is_duplicate to UplinkRepository trait and SqlitePersistence impl</name>
  <files>
    crates/maverick-core/src/ports/uplink_repository.rs
    crates/maverick-adapter-persistence-sqlite/src/persistence/repos.rs
  </files>
  <read_first>
    - crates/maverick-core/src/ports/uplink_repository.rs
    - crates/maverick-adapter-persistence-sqlite/src/persistence/repos.rs
    - crates/maverick-adapter-persistence-sqlite/src/schema.rs
    - crates/maverick-adapter-persistence-sqlite/src/persistence/mod.rs
  </read_first>
  <action>
**uplink_repository.rs** — Add `is_duplicate` to the `UplinkRepository` trait:
```rust
#[async_trait]
pub trait UplinkRepository: Send + Sync {
    async fn append(&self, record: &UplinkRecord) -> AppResult<()>;

    /// Returns true if an uplink with the same (dev_addr, f_cnt) was persisted within the
    /// given time window. Used for multi-gateway duplicate suppression (PROT-06).
    ///
    /// `window_ms` is the look-back window in milliseconds (typically 30_000 for 30 seconds).
    async fn is_duplicate(&self, dev_addr: DevAddr, f_cnt: u32, window_ms: i64) -> AppResult<bool>;
}
```

Also add `DevAddr` to the imports at the top of this file (it comes from `maverick_domain`):
```rust
use maverick_domain::DevAddr;
```

**repos.rs** — Add `is_duplicate` to the `#[async_trait] impl UplinkRepository for SqlitePersistence` block. Follow the exact `run_blocking` + `run_with_busy_retry` two-layer pattern from the existing `append` method:

```rust
async fn is_duplicate(&self, dev_addr: DevAddr, f_cnt: u32, window_ms: i64) -> AppResult<bool> {
    let this = self.clone();
    let key = dev_addr.0 as i64;
    let fcnt = f_cnt as i64;
    this.run_blocking(move |p| {
        p.run_with_busy_retry(|conn| {
            use std::time::{SystemTime, UNIX_EPOCH};
            let now_ms = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis().min(i64::MAX as u128) as i64)
                .unwrap_or(0);
            let cutoff_ms = now_ms - window_ms;
            let sql = schema::sql_check_uplink_dedup();
            let count: i64 = conn.query_row(
                sql.as_str(),
                rusqlite::params![key, fcnt, cutoff_ms],
                |r| r.get(0),
            )?;
            Ok(count > 0)
        })
    })
    .await
}
```

NOTE: The `MemUplinks` in `ingest_uplink.rs` tests also implements `UplinkRepository`. Add a default implementation to the trait OR add a stub `is_duplicate` to `MemUplinks` that always returns `Ok(false)`. Using a trait default method is cleaner:

In `uplink_repository.rs`, you can NOT add a default impl to async_trait methods directly. Instead, add a stub to the `MemUplinks` struct in the test module (Plan C already owns that file — this plan only touches `repos.rs`). The compile error will indicate this. Add `is_duplicate` to `MemUplinks` in `ingest_uplink.rs` test module:
```rust
// In MemUplinks impl UplinkRepository:
async fn is_duplicate(&self, _dev_addr: DevAddr, _f_cnt: u32, _window_ms: i64) -> AppResult<bool> {
    Ok(false) // In-memory stub: never a duplicate in unit tests
}
```

This means `ingest_uplink.rs` must also be modified to add this stub. That file is already modified in Plan C — adding this stub is a small addendum.
  </action>
  <verify>
    <automated>cargo check -p maverick-adapter-persistence-sqlite 2>&1 | grep -E "^error" | head -10</automated>
  </verify>
  <done>
    - `UplinkRepository` trait has `async fn is_duplicate` method signature
    - `SqlitePersistence` implements `is_duplicate` using `sql_check_uplink_dedup()` query
    - `cargo check -p maverick-adapter-persistence-sqlite` passes
    - `cargo check -p maverick-core` passes (MemUplinks stub added)
  </done>
</task>

<task type="auto">
  <name>Task D-2: Wire dedup check into IngestUplink::execute</name>
  <files>
    crates/maverick-core/src/use_cases/ingest_uplink.rs
  </files>
  <read_first>
    - crates/maverick-core/src/use_cases/ingest_uplink.rs
    - crates/maverick-core/src/ports/uplink_repository.rs
  </read_first>
  <action>
In `IngestUplink::execute`, insert the dedup check AFTER MIC verification passes and BEFORE `uplinks.append`. The dedup window is 30 seconds = 30_000 ms (D-12).

Locate the comment `// 6. Persist uplink` and insert BEFORE it:
```rust
// 5b. Duplicate detection (D-10, D-12) — SQLite-backed, survives restarts
// Window: 30_000 ms (30 seconds). Configured as a constant; future config hook goes here.
const DEDUP_WINDOW_MS: i64 = 30_000;
if self.uplinks.is_duplicate(obs.dev_addr, reconstructed_fcnt, DEDUP_WINDOW_MS).await? {
    // Silently discard duplicate — no error, no audit (per D-10: "no audit spam")
    tracing::debug!(
        dev_addr = format!("{:08x}", obs.dev_addr.0),
        f_cnt = reconstructed_fcnt,
        "duplicate uplink discarded"
    );
    return Ok(());
}
```

The `DEDUP_WINDOW_MS` constant should be defined at the module level (outside `execute`) for visibility:
```rust
/// Dedup window: same (dev_addr, f_cnt) within this window is considered a duplicate.
/// Matches multi-gateway scenarios where two gateways forward the same uplink.
const DEDUP_WINDOW_MS: i64 = 30_000;
```

Also add the `MemUplinks` stub for `is_duplicate` in the test module at the bottom of this file:
```rust
// In: impl UplinkRepository for MemUplinks
async fn is_duplicate(
    &self,
    _dev_addr: DevAddr,
    _f_cnt: u32,
    _window_ms: i64,
) -> AppResult<bool> {
    Ok(false) // Unit test stub: never duplicate
}
```

Add import for `DevAddr` in the test module if not already present:
```rust
use maverick_domain::{DevAddr, DevEui, DeviceClass, GatewayEui, RegionId, SessionSnapshot};
```

Add a dedup integration test to verify the silent discard behavior. This test uses `MemUplinks` which always returns `is_duplicate = false`, so a true SQLite dedup test lives in the integration test crate (Plan D verifies the port contract; SQLite behavior is verified in `maverick-integration-tests`).

Add a comment-style test that demonstrates what to test in the integration crate:
```rust
// Integration dedup test lives in:
//   crates/maverick-integration-tests/tests/persistence_sqlite.rs
// See: "dedup_discards_second_frame_within_window"
```
  </action>
  <verify>
    <automated>cargo test -p maverick-core 2>&1 | tail -20</automated>
  </verify>
  <done>
    - `execute` calls `self.uplinks.is_duplicate(obs.dev_addr, reconstructed_fcnt, DEDUP_WINDOW_MS)` after MIC passes
    - On duplicate: `tracing::debug!` then `return Ok(())` with no audit emit
    - `DEDUP_WINDOW_MS = 30_000` constant defined at module level
    - `MemUplinks` has `is_duplicate` stub returning `Ok(false)`
    - `cargo test -p maverick-core` passes all tests
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| dedup query input | `dev_addr` (from trusted session fetch), `reconstructed_fcnt` (u32 from extend_fcnt), `window_ms` (constant) — all bounded |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-01-D-01 | Repudiation | Dedup silent discard (no audit) | accept | Per D-10: "no audit spam" — duplicates from multi-gateway deployments would flood audit log; tracing::debug is sufficient for operator visibility |
| T-01-D-02 | Denial of Service | Dedup query performance | mitigate | `idx_uplinks_dedup` index on `(dev_addr, f_cnt, received_at_ms)` created in Plan A migration; query uses all three columns in WHERE clause |
| T-01-D-03 | Spoofing | Replay via dedup window expiry | accept | 30-second window is for multi-gateway dedup, not replay prevention; replay prevention is FCnt monotonic enforcement (PROT-02) which runs before dedup check |
</threat_model>

<verification>
After both tasks complete:

```bash
cargo test -p maverick-core 2>&1 | tail -20
cargo check -p maverick-adapter-persistence-sqlite
grep -n "is_duplicate" crates/maverick-core/src/ports/uplink_repository.rs
grep -n "is_duplicate" crates/maverick-adapter-persistence-sqlite/src/persistence/repos.rs
grep -n "DEDUP_WINDOW_MS\|is_duplicate" crates/maverick-core/src/use_cases/ingest_uplink.rs
grep -n "sql_check_uplink_dedup" crates/maverick-adapter-persistence-sqlite/src/persistence/repos.rs
```
</verification>

<success_criteria>
- `UplinkRepository` trait has `is_duplicate` method — grep-verifiable
- `SqlitePersistence` implements `is_duplicate` with `sql_check_uplink_dedup()` — grep-verifiable
- `execute` calls `is_duplicate` before `append` — grep-verifiable
- Duplicate path returns `Ok(())` with `tracing::debug!` and NO audit — grep-verifiable
- `DEDUP_WINDOW_MS = 30_000` constant defined — grep-verifiable
- `cargo test -p maverick-core` passes
</success_criteria>

<output>
After completion, create `.planning/phases/01-protocol-correctness/01-D-SUMMARY.md`
</output>
