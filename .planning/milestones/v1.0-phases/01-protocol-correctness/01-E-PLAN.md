---
phase: 01-protocol-correctness
plan: E
type: execute
wave: 2
depends_on:
  - 01-A
files_modified:
  - crates/maverick-adapter-persistence-sqlite/src/persistence/lns_ops.rs
  - crates/maverick-adapter-persistence-sqlite/src/persistence/mod.rs
  - crates/maverick-runtime-edge/src/commands.rs
  - crates/maverick-runtime-edge/src/commands/config.rs
  - crates/maverick-runtime-edge/src/main.rs
autonomous: true
requirements:
  - RELI-01
  - RELI-02

must_haves:
  truths:
    - "No .expect() calls remain inside run_with_busy_retry closures in lns_ops.rs"
    - "SqlitePersistence has a close() method that runs PRAGMA wal_checkpoint(TRUNCATE)"
    - "All process::exit calls in commands.rs and commands/config.rs are replaced with return Err"
    - "main() calls process::exit only after dispatching and after close() on any live SqlitePersistence"
    - "run_setup returns an exit code (i32) rather than calling process::exit internally"
  artifacts:
    - path: "crates/maverick-adapter-persistence-sqlite/src/persistence/lns_ops.rs"
      provides: "parse_hex .expect() → ? propagation"
      contains: "InvalidParameterName"
    - path: "crates/maverick-adapter-persistence-sqlite/src/persistence/mod.rs"
      provides: "SqlitePersistence::close() method"
      contains: "wal_checkpoint"
    - path: "crates/maverick-runtime-edge/src/commands.rs"
      provides: "process::exit removed from handler functions"
      contains: "anyhow::Result"
    - path: "crates/maverick-runtime-edge/src/main.rs"
      provides: "main() maps handler results to exit codes"
      contains: "std::process::exit"
  key_links:
    - from: "crates/maverick-runtime-edge/src/main.rs"
      to: "crates/maverick-adapter-persistence-sqlite/src/persistence/mod.rs"
      via: "main() calls persistence.close() before std::process::exit"
      pattern: "\\.close\\(\\)"
    - from: "crates/maverick-adapter-persistence-sqlite/src/persistence/lns_ops.rs"
      to: "crates/maverick-adapter-persistence-sqlite/src/persistence/busy.rs"
      via: "? propagation inside run_with_busy_retry closures"
      pattern: "InvalidParameterName"
---

<objective>
Remove all Mutex-poisoning `.expect()` calls from `lns_ops.rs` and eliminate `process::exit` from CLI handler functions, replacing both with proper error propagation.

Purpose: RELI-01 (Mutex cannot be permanently poisoned) and RELI-02 (clean shutdown with WAL checkpoint) must ship in Phase 1 before supervision (Phase 4) is meaningful. These are independent of the crypto work in Plans B/C/D and can run in parallel with Plan B.

Output: `lns_ops.rs` with all 11 `.expect()` calls replaced by `?`-propagation. `SqlitePersistence::close()` for WAL checkpoint. `commands.rs` and `commands/config.rs` with `process::exit` replaced by `Result` returns. `main()` dispatches results to `std::process::exit`.
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
  <name>Task E-1: Fix .expect() in lns_ops.rs and add SqlitePersistence::close()</name>
  <files>
    crates/maverick-adapter-persistence-sqlite/src/persistence/lns_ops.rs
    crates/maverick-adapter-persistence-sqlite/src/persistence/mod.rs
  </files>
  <read_first>
    - crates/maverick-adapter-persistence-sqlite/src/persistence/lns_ops.rs
    - crates/maverick-adapter-persistence-sqlite/src/persistence/mod.rs
    - crates/maverick-adapter-persistence-sqlite/src/persistence/busy.rs
  </read_first>
  <action>
**lns_ops.rs** — Replace ALL `.expect()` calls inside `apply_lns_config_inner` with `?`-propagation. The function returns `Result<(), rusqlite::Error>` so all errors must convert to `rusqlite::Error`. Use `rusqlite::Error::InvalidParameterName(msg.to_string())` as the conversion target (per D-16 locked decision).

The parse functions (`parse_hex_dev_eui`, `parse_hex_dev_addr`, `parse_hex_16`, `parse_hex_32`) already return `Result<T, E>` where `E` is a string or custom type. The fix is to replace each `.expect("validated lns config")` and `.expect("validated")` with `.map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?`.

Full list of sites to fix (per CONTEXT.md D-16 and RESEARCH.md verified locations):

Line 288 area:
```rust
// Before:
let dev_eui_b = parse_hex_dev_eui(&d.dev_eui).expect("validated lns config");
// After:
let dev_eui_b = parse_hex_dev_eui(&d.dev_eui)
    .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
```

Lines 295-296 area (dev_addr parsing for ABP):
```rust
// Before:
let u = parse_hex_dev_addr(d.dev_addr.as_ref().expect("validated abp"))
    .expect("validated");
// After (two steps):
let addr_str = d.dev_addr.as_ref()
    .ok_or_else(|| rusqlite::Error::InvalidParameterName("abp dev_addr missing".to_string()))?;
let u = parse_hex_dev_addr(addr_str)
    .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
```

Lines 312-313 area (join_eui, app_key):
```rust
// Before:
let j = parse_hex_16(&k.join_eui).expect("validated lns config");
let ak = parse_hex_32(&k.app_key).expect("validated lns config");
// After:
let j = parse_hex_16(&k.join_eui)
    .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
let ak = parse_hex_32(&k.app_key)
    .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
```

Line 317 area (nwk_key optional):
```rust
// Before:
let nk = k.nwk_key.as_ref().map(|s| parse_hex_32(s).expect("validated lns config"));
// After:
let nk = k.nwk_key.as_ref()
    .map(|s| parse_hex_32(s).map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string())))
    .transpose()?;
```

Lines 327, 332 area (apps_key, nwks_key optional):
```rust
// Before:
let a = abp.apps_key.as_ref()
    .filter(|s| !s.trim().is_empty())
    .map(|s| parse_hex_32(s).expect("validated"));
let n = abp.nwks_key.as_ref()
    .filter(|s| !s.trim().is_empty())
    .map(|s| parse_hex_32(s).expect("validated"));
// After:
let a = abp.apps_key.as_ref()
    .filter(|s| !s.trim().is_empty())
    .map(|s| parse_hex_32(s).map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string())))
    .transpose()?;
let n = abp.nwks_key.as_ref()
    .filter(|s| !s.trim().is_empty())
    .map(|s| parse_hex_32(s).map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string())))
    .transpose()?;
```

Lines 382, 399-400 area (second device loop for ABP devices in sessions sync block):
Apply the same pattern as above for any `.expect()` calls in the ABP device session insertion loop.

Read the full file before making changes — the line numbers in CONTEXT.md are approximate. Do a full read and fix ALL `.expect()` calls inside `apply_lns_config_inner` and any other closure passed to `run_with_busy_retry`. Do NOT touch `.expect()` calls outside of Mutex lock scopes (e.g., in `doc.validate()` callers — those are outside the lock).

Also check `lns_approve_device` and `lns_upsert_pending` for any `.expect()` inside their `run_with_busy_retry` closures and fix those too.

**mod.rs** — Add `SqlitePersistence::close()` method:
```rust
impl SqlitePersistence {
    // ... existing open() and run_blocking() methods ...

    /// Checkpoint the SQLite WAL before process exit (D-19).
    ///
    /// Call from main() before std::process::exit to ensure all committed
    /// WAL frames are flushed to the main database file.
    ///
    /// Note: rusqlite 0.33 Connection::drop does NOT trigger WAL checkpoint automatically.
    pub fn close(self) -> AppResult<()> {
        // Only checkpoint if this is the last Arc holder
        if std::sync::Arc::strong_count(&self.inner) == 1 {
            let guard = self.inner.conn.lock().map_err(|_| {
                AppError::Infrastructure("mutex_poisoned: cannot checkpoint on close".to_string())
            })?;
            guard
                .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
                .map_err(|e| {
                    AppError::Infrastructure(format!("wal_checkpoint on close: {e}"))
                })?;
        }
        Ok(())
    }
}
```
  </action>
  <verify>
    <automated>grep -n "\.expect(" crates/maverick-adapter-persistence-sqlite/src/persistence/lns_ops.rs | grep -v "//.*expect\|#\[" | head -10</automated>
  </verify>
  <done>
    - No `.expect()` calls remain inside `apply_lns_config_inner` or other `run_with_busy_retry` closures in `lns_ops.rs`
    - All replaced with `.map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?`
    - `SqlitePersistence::close()` method exists in `mod.rs` with `PRAGMA wal_checkpoint(TRUNCATE)`
    - `cargo check -p maverick-adapter-persistence-sqlite` passes
    - grep of lns_ops.rs for `.expect(` returns no results (excluding comments)
  </done>
</task>

<task type="auto">
  <name>Task E-2: Replace process::exit in commands with Result propagation</name>
  <files>
    crates/maverick-runtime-edge/src/commands.rs
    crates/maverick-runtime-edge/src/commands/config.rs
    crates/maverick-runtime-edge/src/main.rs
  </files>
  <read_first>
    - crates/maverick-runtime-edge/src/commands.rs
    - crates/maverick-runtime-edge/src/commands/config.rs
    - crates/maverick-runtime-edge/src/main.rs
  </read_first>
  <action>
This is a mechanical refactor across three files. The pattern is uniform: replace each `std::process::exit(N)` in handler functions with `return Err(anyhow::anyhow!("message"))`, update handler return types to `anyhow::Result<()>`, and update `main()` to dispatch results.

**commands/config.rs** — Read the full file. Every function that currently calls `std::process::exit(N)` must:
1. Have its return type changed from `()` to `anyhow::Result<()>`
2. Have each `std::process::exit(N)` replaced with `return Err(anyhow::anyhow!("{message}"))` where `{message}` is the `eprintln!` string that precedes the exit call
3. Have each early-return-before-exit replaced to return `Ok(())`
4. Have a final `Ok(())` at the end

Example transform:
```rust
// Before:
pub(crate) fn run_config_init(config_path: PathBuf, force: bool) {
    if config_path.exists() && !force {
        eprintln!("config file already exists at {}; use --force to overwrite", config_path.display());
        std::process::exit(2);
    }
    // ...
    match write_result {
        Ok(_) => {},
        Err(e) => {
            eprintln!("failed to write config: {e}");
            std::process::exit(1);
        }
    }
}

// After:
pub(crate) fn run_config_init(config_path: PathBuf, force: bool) -> anyhow::Result<()> {
    if config_path.exists() && !force {
        return Err(anyhow::anyhow!(
            "config file already exists at {}; use --force to overwrite",
            config_path.display()
        ));
    }
    // ...
    write_result.map_err(|e| anyhow::anyhow!("failed to write config: {e}"))?;
    Ok(())
}
```

Apply this pattern to ALL functions in `config.rs` that call `process::exit`.

**commands.rs** — Apply the same pattern to all handler functions in this file.

Special case for `run_setup`: This function shells out to a subprocess and propagates the child's exit code. The correct pattern is to return `i32` (the exit code) rather than `Result`. Use a dedicated return type:
```rust
pub(crate) fn run_setup(non_interactive: bool) -> i32 {
    // ...
    if !non_interactive && (...) {
        eprintln!("setup requires an interactive terminal...");
        return 2;
    }
    // ...
    match status.code() {
        Some(code) => code,
        None => 1,
    }
}
```

For ALL other async handlers (`run_status`, `run_health`, `run_radio_downlink_probe`, etc.) that currently call `process::exit`: change return type to `anyhow::Result<()>` and replace exit calls with `Err(anyhow::anyhow!(...))`.

**main.rs** — Update `main()` to:
1. Capture `Result<(), anyhow::Error>` from each command dispatch
2. Handle `run_setup` separately (returns `i32`)
3. Call `std::process::exit` with the appropriate code ONLY in `main()`, after all async work completes

```rust
#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    let db_file = EDGE_DB_FILENAME;

    let exit_code: i32 = match cli.command {
        Commands::Setup { non_interactive } => run_setup(non_interactive),

        Commands::Status => {
            match run_status(cli.data_dir, db_file).await {
                Ok(()) => 0,
                Err(e) => { eprintln!("error: {e:#}"); 1 }
            }
        }

        Commands::Health => {
            match run_health(cli.data_dir, db_file).await {
                Ok(()) => 0,
                Err(e) => { eprintln!("error: {e:#}"); 1 }
            }
        }

        Commands::RecentErrors { lines } => {
            match run_recent_errors(lines) {
                Ok(()) => 0,
                Err(e) => { eprintln!("error: {e:#}"); 1 }
            }
        }

        Commands::Probe { summary } => {
            match run_probe(summary) {
                Ok(()) => 0,
                Err(e) => { eprintln!("error: {e:#}"); 1 }
            }
        }

        Commands::StoragePolicy { profile } => {
            match run_storage_policy(profile.into()) {
                Ok(()) => 0,
                Err(e) => { eprintln!("error: {e:#}"); 1 }
            }
        }

        Commands::StoragePressure => {
            match run_storage_pressure(cli.data_dir, db_file).await {
                Ok(()) => 0,
                Err(e) => { eprintln!("error: {e:#}"); 1 }
            }
        }

        Commands::Radio { cmd } => match cmd {
            RadioCmd::DownlinkProbe { host, port } => {
                match run_radio_downlink_probe(host, port).await {
                    Ok(()) => 0,
                    Err(e) => { eprintln!("error: {e:#}"); 1 }
                }
            }
            RadioCmd::IngestOnce { bind, timeout_ms } => {
                match run_radio_ingest_once(cli.data_dir, db_file, bind, timeout_ms).await {
                    Ok(()) => 0,
                    Err(e) => { eprintln!("error: {e:#}"); 1 }
                }
            }
            RadioCmd::IngestLoop { bind, read_timeout_ms, max_messages } => {
                match run_radio_ingest_supervised(cli.data_dir, db_file, bind, read_timeout_ms, max_messages).await {
                    Ok(()) => 0,
                    Err(e) => { eprintln!("error: {e:#}"); 1 }
                }
            }
        },

        Commands::Config { cmd } => match cmd {
            ConfigCmd::Init { force, config_path } => {
                match config::run_config_init(config_path, force) {
                    Ok(()) => 0,
                    Err(e) => { eprintln!("error: {e:#}"); 1 }
                }
            }
            // ... same pattern for all other ConfigCmd variants
        },
    };

    // Tokio runtime drops here, giving in-flight spawn_blocking tasks a chance to complete.
    // WAL checkpoint: SqlitePersistence instances are dropped when command handlers return.
    // For commands that hold a long-lived SqlitePersistence (IngestLoop), call close() before returning Ok(()).
    std::process::exit(exit_code);
}
```

IMPORTANT: Read the full `main.rs` dispatch block before editing — there may be more commands than shown above. Apply the pattern to every arm.

For ingest loop handlers (`run_radio_ingest_supervised`): if they hold a `SqlitePersistence`, update them to call `persistence.close()` before returning — pass the `Arc<SqlitePersistence>` or owned `SqlitePersistence` through and close it in the handler's error/success path.

NOTE: `anyhow` is already a dependency of `maverick-runtime-edge` (it is used in the existing `ingest.rs`). Verify with `cargo check`.
  </action>
  <verify>
    <automated>grep -rn "std::process::exit" crates/maverick-runtime-edge/src/commands.rs crates/maverick-runtime-edge/src/commands/config.rs 2>/dev/null | head -5</automated>
  </verify>
  <done>
    - No `std::process::exit` calls remain in `commands.rs` or `commands/config.rs` (only in `main.rs`)
    - All handler functions in `commands.rs` and `commands/config.rs` return `anyhow::Result<()>` (or `i32` for `run_setup`)
    - `main.rs` calls `std::process::exit(exit_code)` at the end after all dispatch
    - `cargo check -p maverick-runtime-edge` passes
    - `cargo test --workspace` passes (integration tests still pass after the signature changes)
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| Mutex<Connection> in lns_ops.rs | SQLite operations inside run_with_busy_retry; panics here poison the mutex |
| process exit in async context | std::process::exit in an async fn abandons in-flight spawn_blocking SQLite writes |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-01-E-01 | Denial of Service | Mutex poison from .expect() panic | mitigate | All .expect() inside run_with_busy_retry replaced with ? using InvalidParameterName; eliminates panic source in debug/test builds |
| T-01-E-02 | Denial of Service | WAL not checkpointed on exit | mitigate | SqlitePersistence::close() calls PRAGMA wal_checkpoint(TRUNCATE); main() drops runtime after dispatch; write-ahead log flushed |
| T-01-E-03 | Tampering | process::exit abandons in-flight writes | mitigate | Handlers return Result; main() calls process::exit after tokio runtime drop; spawn_blocking tasks complete before process exits |
</threat_model>

<verification>
After both tasks complete:

```bash
cargo check -p maverick-adapter-persistence-sqlite
cargo check -p maverick-runtime-edge
grep -c "\.expect(" crates/maverick-adapter-persistence-sqlite/src/persistence/lns_ops.rs
grep -n "wal_checkpoint" crates/maverick-adapter-persistence-sqlite/src/persistence/mod.rs
grep -rn "std::process::exit" crates/maverick-runtime-edge/src/commands.rs crates/maverick-runtime-edge/src/commands/config.rs
```

Expected: `.expect(` count is 0 (or only in comments), `wal_checkpoint` present in mod.rs, no `process::exit` in commands/*.
</verification>

<success_criteria>
- `grep "\.expect(" lns_ops.rs` returns 0 results (excluding comments) — verifiable
- `SqlitePersistence::close()` method exists with `PRAGMA wal_checkpoint(TRUNCATE)` — grep-verifiable
- No `std::process::exit` in `commands.rs` or `commands/config.rs` — grep-verifiable
- `main()` calls `std::process::exit(exit_code)` as the last statement — grep-verifiable
- `cargo check -p maverick-adapter-persistence-sqlite` passes
- `cargo check -p maverick-runtime-edge` passes
</success_criteria>

<output>
After completion, create `.planning/phases/01-protocol-correctness/01-E-SUMMARY.md`
</output>
