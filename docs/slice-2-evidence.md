# Slice 2 evidence — durable persistence (incremental)

Date: 2026-04-09

This documents what was implemented for **Slice 2 (persistence + retention)**. It is **not** a claim that all of `docs/02-delivery-checklist.md` is satisfied for v1.

## What shipped

1. **Adapter crate** [`crates/maverick-adapter-persistence-sqlite`](../crates/maverick-adapter-persistence-sqlite): SQLite implementation of `SessionRepository`, `UplinkRepository`, `AuditSink`, and `StoragePressureSource` from `maverick-core`. No SQLite (or other DB) dependency was added to `maverick-core`. DDL lives in `schema.sql`; table/column identifiers and dynamic SQL builders live in the public `schema` module with integration coverage that DDL and `names::*` stay aligned.
2. **Retention**: Per-tier approximate caps from `StoragePolicy` — uplinks (telemetry), audit (operational), sessions (critical, LRU by `updated_at_ms`). Optional **circular hard-limit trim** when `circular_at_hard_limit` is true and on-disk DB size exceeds ~98% of configured total disk hint.
3. **Busy handling**: `busy_timeout` plus bounded retries on `SQLITE_BUSY` in the adapter write path.
4. **CLI** (`maverick-edge`): global `--data-dir` / `MAVERICK_DATA_DIR` (default `data`), DB path `maverick.db`. `status` and `health` include storage when the DB file exists; new `storage-pressure` subcommand prints `StoragePressureSnapshot` JSON.
5. **Install profiles**: `InstallProfile::default_storage_policy()` now uses distinct tier caps for `Constrained` and `HighCapacity` (not only circular default).
6. **Tests** (see `docs/05-test-program.md`): integration coverage in [`crates/maverick-integration-tests/tests/persistence_sqlite.rs`](../crates/maverick-integration-tests/tests/persistence_sqlite.rs) — ingest + reopen, telemetry prune, concurrent `BEGIN IMMEDIATE` vs append.

## Verification commands (local / CI)

```bash
cargo test --workspace
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all --check
```

## Known gaps vs full v1 checklist

- No long-running edge daemon loop yet; persistence is validated via integration tests and CLI-open paths.
- Rotating structured logs and diagnostics journal remain future work (Slice 4).
- Soak / burst / full fault matrix not complete; only SQLite busy covered in automated tests so far.
