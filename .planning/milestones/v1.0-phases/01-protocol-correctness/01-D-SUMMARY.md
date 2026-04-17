---
plan: 01-D
phase: 01-protocol-correctness
status: complete
tasks_completed: 2
tasks_total: 2
requirements_covered:
  - PROT-06
---

## Summary

Added `is_duplicate` to `UplinkRepository` trait and `SqlitePersistence` implementation. Wired a 30-second dedup window into `IngestUplink::execute` — duplicate frames (same dev_addr + f_cnt within 30 s) are discarded silently with `tracing::debug!` and no audit noise.

## Tasks

### Task D-1: is_duplicate trait method + SqlitePersistence impl

**uplink_repository.rs** — Added to `UplinkRepository` trait:
```rust
async fn is_duplicate(&self, dev_addr: DevAddr, f_cnt: u32, window_ms: i64) -> AppResult<bool>;
```

**repos.rs** — `SqlitePersistence` impl follows the existing `run_blocking` + `run_with_busy_retry` pattern:
- Computes `now_ms - window_ms` as cutoff
- Uses `sql_check_uplink_dedup()` → `SELECT COUNT(*) WHERE dev_addr=?1 AND f_cnt=?2 AND received_at_ms>=?3`
- Returns `count > 0`

### Task D-2: Wire dedup into execute

In `ingest_uplink.rs`:
- Added `DEDUP_WINDOW_MS: i64 = 30_000` module-level constant
- Added dedup check after MIC passes, before `uplinks.append`
- Duplicate path: `tracing::debug!` → `return Ok(())` — no error, no audit
- `MemUplinks` test stub: `is_duplicate` always returns `Ok(false)`

## Key Files

### Modified
- `crates/maverick-core/src/ports/uplink_repository.rs` — `is_duplicate` method on trait
- `crates/maverick-adapter-persistence-sqlite/src/persistence/repos.rs` — SQLite impl
- `crates/maverick-core/src/use_cases/ingest_uplink.rs` — dedup check + MemUplinks stub

## Self-Check: PASSED

- `grep "is_duplicate" uplink_repository.rs` ✓
- `grep "is_duplicate" repos.rs` ✓
- `grep "DEDUP_WINDOW_MS\|is_duplicate" ingest_uplink.rs` ✓
- `cargo test --workspace` all pass ✓
