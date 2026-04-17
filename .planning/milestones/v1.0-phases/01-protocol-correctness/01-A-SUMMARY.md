---
phase: 01-protocol-correctness
plan: A
subsystem: database
tags: [lorawan, sqlite, session-keys, mic-verification, fcnt, schema-migration, rusqlite]

# Dependency graph
requires: []
provides:
  - SessionSnapshot with nwk_s_key:[u8;16] and app_s_key:[u8;16] fields
  - UplinkObservation with f_cnt:u16, wire_mic:[u8;4], phy_without_mic:Vec<u8>
  - UplinkRecord with received_at_ms:i64 and payload_decrypted:Option<Vec<u8>>
  - SQLite sessions table with nwk_s_key/app_s_key BLOB NOT NULL columns
  - SQLite uplinks table with received_at_ms INTEGER NOT NULL, payload_decrypted BLOB, idx_uplinks_dedup index
  - sql_check_uplink_dedup() builder function
  - row_to_session reads key bytes at column indices 7 and 8
affects:
  - 01-B (FCnt 32-bit fix — uses updated UplinkObservation.f_cnt:u16)
  - 01-C (MIC verification — uses nwk_s_key, app_s_key, wire_mic, phy_without_mic)
  - 01-D (Dedup — uses received_at_ms, idx_uplinks_dedup, sql_check_uplink_dedup)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "BLOB-to-array pattern: Vec<u8> from SQLite row -> copy_from_slice into [u8;N] with length guard"
    - "Best-effort ALTER TABLE migration: let _ = conn.execute(...) — silently ignored for existing columns"
    - "SQL builder functions in schema.rs: all query strings assembled from named column constants"

key-files:
  created: []
  modified:
    - crates/maverick-domain/src/session.rs
    - crates/maverick-core/src/ports/radio_transport.rs
    - crates/maverick-core/src/ports/uplink_repository.rs
    - crates/maverick-adapter-persistence-sqlite/src/schema.sql
    - crates/maverick-adapter-persistence-sqlite/src/schema.rs
    - crates/maverick-adapter-persistence-sqlite/src/persistence/sql.rs
    - crates/maverick-adapter-persistence-sqlite/src/persistence/repos.rs
    - crates/maverick-adapter-persistence-sqlite/src/persistence/lns_ops.rs
    - crates/maverick-core/src/protocol/lorawan_10x_class_a.rs
    - crates/maverick-core/src/use_cases/ingest_uplink.rs

key-decisions:
  - "f_cnt is u16 in UplinkObservation (wire value); cast to u32 at protocol comparison boundary (Plan B owns full rollover logic)"
  - "Session keys sourced from lns-config ABP fields in apply_lns_config, falling back to existing session then zero — zero-key sessions are inert until config load provides real keys"
  - "row_to_session uses unwrap_or_default() for key columns (indices 7/8) to tolerate rows from migrated DBs where ALTER TABLE added nullable columns"

patterns-established:
  - "BLOB key pattern: store [u8;16] keys as BLOB in SQLite; read back as Vec<u8> and copy_from_slice with length guard"
  - "Migration pattern: migrate_*_v2 functions called from init_schema; best-effort ALTER TABLE with let _ = ... to handle both new and existing DBs"

requirements-completed:
  - PROT-03
  - PROT-04
  - CORE-02

# Metrics
duration: 35min
completed: 2026-04-16
---

# Phase 01 Plan A: Domain Model and Schema Foundations Summary

**SessionSnapshot extended with AES-128 NwkSKey/AppSKey fields, UplinkObservation f_cnt narrowed to u16 with wire_mic/phy_without_mic added, SQLite sessions/uplinks DDL updated with key columns and dedup index**

## Performance

- **Duration:** ~35 min
- **Started:** 2026-04-16T00:00:00Z
- **Completed:** 2026-04-16
- **Tasks:** 2
- **Files modified:** 10

## Accomplishments

- Extended `SessionSnapshot` with `nwk_s_key: [u8; 16]` and `app_s_key: [u8; 16]` — unblocks MIC verification (Plan C)
- Changed `UplinkObservation.f_cnt` from `u32` to `u16` (wire-level) and added `wire_mic`/`phy_without_mic` fields — provides raw bytes for MIC verifier
- Added `received_at_ms: i64` and `payload_decrypted: Option<Vec<u8>>` to `UplinkRecord` — enables dedup window query (Plan D) and future payload decryption
- Updated SQLite DDL: `sessions` gains `nwk_s_key BLOB NOT NULL` and `app_s_key BLOB NOT NULL`; `uplinks` gains `received_at_ms INTEGER NOT NULL`, `payload_decrypted BLOB`, and `idx_uplinks_dedup` composite index
- Added `migrate_sessions_v2` and `migrate_uplinks_v2` best-effort ALTER TABLE migrations for existing dev databases
- Updated `row_to_session`, `sql_select_session_by_dev_addr`, `sql_upsert_session`, `sql_insert_uplink`, and all call sites

## Task Commits

1. **Task A-1: Extend domain model and port types** - `5e069af` (feat)
2. **Task A-2: Schema DDL, column constants, and row_to_session update** - `08c316e` (feat)

## Files Created/Modified

- `crates/maverick-domain/src/session.rs` — Added `nwk_s_key` and `app_s_key` fields to `SessionSnapshot`
- `crates/maverick-core/src/ports/radio_transport.rs` — Changed `f_cnt` to `u16`, added `wire_mic` and `phy_without_mic` to `UplinkObservation`
- `crates/maverick-core/src/ports/uplink_repository.rs` — Added `received_at_ms` and `payload_decrypted` to `UplinkRecord`
- `crates/maverick-adapter-persistence-sqlite/src/schema.sql` — Updated sessions and uplinks DDL, added dedup index
- `crates/maverick-adapter-persistence-sqlite/src/schema.rs` — Added `NWK_S_KEY`, `APP_S_KEY`, `RECEIVED_AT_MS`, `PAYLOAD_DECRYPTED` constants; updated SQL builder functions; added `sql_check_uplink_dedup()`
- `crates/maverick-adapter-persistence-sqlite/src/persistence/sql.rs` — Updated `init_schema`, added `migrate_sessions_v2`/`migrate_uplinks_v2`, updated `row_to_session`
- `crates/maverick-adapter-persistence-sqlite/src/persistence/repos.rs` — Updated `upsert` and `append` params for new columns
- `crates/maverick-adapter-persistence-sqlite/src/persistence/lns_ops.rs` — Fixed both `SessionSnapshot` constructors and `sql_upsert_session` params
- `crates/maverick-core/src/protocol/lorawan_10x_class_a.rs` — Fixed `f_cnt as u32` comparison cast and test helper types
- `crates/maverick-core/src/use_cases/ingest_uplink.rs` — Fixed `UplinkRecord` constructor call sites and test helpers

## Decisions Made

- `f_cnt` is `u16` in `UplinkObservation` (wire value); plan specifies reconstruction to 32-bit happens in the protocol module (Plan B owns rollover logic)
- Session keys in `lns_ops.rs` `apply_lns_config` are sourced from ABP config fields if present, then fall back to existing session keys, then zero — zero-key sessions remain inert until a config load provides real keys
- `row_to_session` uses `unwrap_or_default()` for key column reads at indices 7/8 to tolerate migrated DBs where ALTER TABLE added nullable columns

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Fixed transitive compilation failures in maverick-core call sites**
- **Found during:** Task A-2 verification (`cargo check -p maverick-adapter-persistence-sqlite`)
- **Issue:** The plan noted that `ingest_uplink.rs` and `lorawan_10x_class_a.rs` would break and "be fixed in Plan C/B", but the adapter crate transitively depends on `maverick-core`, so the adapter check also failed. The plan's done criterion ("cargo check -p maverick-adapter-persistence-sqlite passes") was unreachable without fixing these call sites.
- **Fix:** Updated `ingest_uplink.rs` `UplinkRecord` constructor to use `f_cnt as u32`, added `received_at_ms: 0` (TODO stub), `payload_decrypted: None` (TODO stub), and `f_cnt as u32` for the session update. Fixed `lorawan_10x_class_a.rs` comparison cast and test helper signatures. Updated test `SessionSnapshot` constructors in both files to include `nwk_s_key`/`app_s_key`.
- **Files modified:** `crates/maverick-core/src/use_cases/ingest_uplink.rs`, `crates/maverick-core/src/protocol/lorawan_10x_class_a.rs`
- **Verification:** `cargo check -p maverick-domain` and `cargo check -p maverick-adapter-persistence-sqlite` both pass with no errors
- **Committed in:** `08c316e` (Task A-2 commit)

**2. [Rule 2 - Missing Critical] Fixed lns_ops.rs SessionSnapshot constructors for key fields**
- **Found during:** Task A-2 verification
- **Issue:** `lns_ops.rs` had two `SessionSnapshot` literal constructors (in `lns_approve_device` and `apply_lns_config_inner`) that were missing the new `nwk_s_key`/`app_s_key` fields, and their corresponding `sql_upsert_session` params calls were also missing params ?9 and ?10.
- **Fix:** Added key sourcing logic (from existing session / config / zero fallback) to both sites and passed `&session.nwk_s_key[..]`/`&session.app_s_key[..]` as params ?9 and ?10.
- **Files modified:** `crates/maverick-adapter-persistence-sqlite/src/persistence/lns_ops.rs`
- **Verification:** `cargo check -p maverick-adapter-persistence-sqlite` passes
- **Committed in:** `08c316e` (Task A-2 commit)

---

**Total deviations:** 2 auto-fixed (1 blocking, 1 missing critical)
**Impact on plan:** Both auto-fixes were necessary to satisfy the plan's done criterion. No scope creep — all fixes are direct consequences of the domain type changes.

## Known Stubs

| Stub | File | Line | Reason |
|------|------|------|--------|
| `received_at_ms: 0` | `crates/maverick-core/src/use_cases/ingest_uplink.rs` | 51 | Placeholder until Plan C threads `now_ms()` through the ingest path |
| `payload_decrypted: None` | `crates/maverick-core/src/use_cases/ingest_uplink.rs` | 54 | Placeholder until Plan C wires AppSKey decryption |

These stubs do not prevent Plan A's goal (domain/schema foundations). They will be resolved in Plan C.

## Issues Encountered

None beyond the auto-fixed call-site breakages documented above.

## Next Phase Readiness

- Plan B (FCnt 32-bit) can now safely update its call sites — `f_cnt: u16` type is stable
- Plan C (MIC verification) has all required fields: `nwk_s_key`, `app_s_key`, `wire_mic`, `phy_without_mic`
- Plan D (Dedup) has `received_at_ms`, `idx_uplinks_dedup`, and `sql_check_uplink_dedup()`
- `cargo check -p maverick-domain` and `cargo check -p maverick-adapter-persistence-sqlite` pass
- `cargo test --workspace` will still fail due to downstream call-site breakage (expected per plan; Plans B and C own the remaining fixes)

---
*Phase: 01-protocol-correctness*
*Completed: 2026-04-16*
