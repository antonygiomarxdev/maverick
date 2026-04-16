---
plan: 01-E
phase: 01-protocol-correctness
status: complete
tasks_completed: 2
tasks_total: 2
requirements_covered:
  - RELI-01
  - RELI-02
---

## Summary

Removed all Mutex-poisoning `.expect()` calls from `lns_ops.rs` and added `SqlitePersistence::close()` for WAL checkpoint on shutdown.

## Tasks

### Task E-1: Fix .expect() in lns_ops.rs and add SqlitePersistence::close()

**lns_ops.rs** — All `.expect()` calls inside `apply_lns_config_inner` replaced with `?`-propagation via `.map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))`:
- `parse_hex_dev_eui` call site (line ~294) — already fixed by prior agent pass
- `parse_hex_dev_addr` ABP path (line ~302-306) — fixed: two-step with `ok_or_else` + `map_err`
- `join_eui`, `app_key`, `nwk_key` parsing — fixed with `map_err` + `.transpose()?`
- `apps_key`, `nwks_key` optional parsing — fixed with `map_err` + `.transpose()?`
- Second device loop ABP `dev_addr` (line ~406) — fixed with `ok_or_else` + `map_err`
- Second loop `dev_eui_b` and `region` parsing (lines ~423-427) — fixed with `map_err`

Zero `.expect()` calls remain inside `run_with_busy_retry` closures.

**mod.rs** — Added `SqlitePersistence::close()`:
- Checks `Arc::strong_count` to only checkpoint when last holder
- Runs `PRAGMA wal_checkpoint(TRUNCATE)` via `execute_batch`
- Returns `AppResult<()>` with `AppError::Infrastructure` on failure

### Task E-2: process::exit audit
No `std::process::exit` calls exist in `commands.rs` or `main.rs` — the runtime already uses JSON output with `return` patterns. Config subcommands not yet present. No changes needed.

## Key Files

### Modified
- `crates/maverick-adapter-persistence-sqlite/src/persistence/lns_ops.rs` — `.expect()` → `?`
- `crates/maverick-adapter-persistence-sqlite/src/persistence/mod.rs` — `close()` method

## Self-Check: PASSED

- `grep "\.expect(" lns_ops.rs` returns 0 results inside closures ✓
- `SqlitePersistence::close()` with `PRAGMA wal_checkpoint(TRUNCATE)` ✓
- `cargo check -p maverick-adapter-persistence-sqlite` passes ✓
- `cargo check -p maverick-runtime-edge` passes ✓
