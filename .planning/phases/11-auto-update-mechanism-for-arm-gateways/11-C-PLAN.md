---
phase: 11
plan: C
subsystem: update
wave: 1
depends_on: []
type: execute
files_modified:
  - crates/maverick-runtime-edge/src/main.rs (update CLI subcommands)
  - crates/maverick-runtime-edge/src/update/cli.rs (new file)
autonomous: true
requirements: []
---

<objective>
Add CLI subcommands for the update mechanism: `update check`, `update status`, and `update history`. These allow operators to manually trigger updates and view update status/history.

**Out of scope:** Update logic implementation (Plan B), systemd service/timer setup (Plan A)
</objective>

<read_first>
- .planning/phases/11-auto-update-mechanism-for-arm-gateways/11-CONTEXT.md (update decisions)
- .planning/phases/05-tui-device-management/05-CONTEXT.md (CLI patterns)
- crates/maverick-runtime-edge/src/main.rs (existing CLI structure)
</read_first>

<action>
Create `crates/maverick-runtime-edge/src/update/cli.rs` — CLI commands for update:

```rust
//! Update CLI commands

use crate::update::{UpdateConfig, UpdateError};

/// Run update check manually
pub fn check() -> Result<(), UpdateError> {
    let config = UpdateConfig::load()?;
    let current = UpdateConfig::current_version()?;
    
    println!("Current version: {}", current);
    println!("Update mode: {}", match config.mode {
        crate::update::UpdateMode::Release => "release",
        crate::update::UpdateMode::Dev => "dev",
    });
    
    match config.mode {
        crate::update::UpdateMode::Release => {
            if let Some(release_url) = &config.release_url {
                println!("Release URL: {}", release_url);
                
                match config.check_release_update() {
                    Ok(Some(new_version)) => {
                        println!("");
                        println!("Update available: {} -> {}", current, new_version);
                        println!("Run 'systemctl start maverick-update.service' to apply");
                    }
                    Ok(None) => {
                        println!("No update available.");
                    }
                    Err(e) => {
                        eprintln!("Update check failed: {}", e);
                        return Err(e);
                    }
                }
            } else {
                println!("No release_url configured.");
                println!("Set [update] section in /etc/maverick/maverick.toml");
            }
        }
        crate::update::UpdateMode::Dev => {
            println!("Dev mode: git pull + cargo build");
            println!("Run 'systemctl start maverick-update.service' to trigger");
        }
    }
    
    Ok(())
}

/// Show update status
pub fn status() -> Result<(), UpdateError> {
    let current = UpdateConfig::current_version()?;
    println!("Current version: {}", current);
    
    // Check last update from journal
    let output = std::process::Command::new("journalctl")
        .args([
            "-u", "maverick-update.service",
            "-n", "1",
            "--no-pager",
            "--output=short-iso"
        ])
        .output();
    
    match output {
        Ok(output) if output.status.success() => {
            let last_update = String::from_utf8_lossy(&output.stdout).trim();
            if !last_update.is_empty() {
                println!("Last update: {}", last_update);
            } else {
                println!("Last update: never (or no updates logged yet)");
            }
        }
        _ => {
            println!("Last update: unknown (journal unavailable)");
        }
    }
    
    // Show backup count
    let config = UpdateConfig::load()?;
    if let Ok(entries) = std::fs::read_dir(&config.backup_dir) {
        let count = entries.filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().starts_with("maverick-edge-"))
            .count();
        println!("Backups available: {}", count);
    }
    
    // Show timer status
    let timer_output = std::process::Command::new("systemctl")
        .args(["is-active", "maverick-update.timer"])
        .output();
    
    if let Ok(output) = timer_output {
        let active = String::from_utf8_lossy(&output.stdout).trim();
        println!("Update timer: {}", active);
    }
    
    let enabled_output = std::process::Command::new("systemctl")
        .args(["is-enabled", "maverick-update.timer"])
        .output();
    
    if let Ok(output) = enabled_output {
        let enabled = String::from_utf8_lossy(&output.stdout).trim();
        println!("Timer enabled: {}", enabled);
    }
    
    Ok(())
}

/// Show update history from journal
pub fn history(n: usize) -> Result<(), UpdateError> {
    let lines = if n == 0 { 10 } else { n };
    
    let output = std::process::Command::new("journalctl")
        .args([
            "-u", "maverick-update.service",
            "-n", lines.to_string().as_str(),
            "--no-pager"
        ])
        .output()
        .map_err(|e| UpdateError::Command(format!("journalctl failed: {}", e)))?;
    
    if !output.status.success() {
        return Err(UpdateError::Command(format!(
            "journalctl exited with {}",
            output.status.code().unwrap_or(-1)
        )));
    }
    
    let history = String::from_utf8_lossy(&output.stdout);
    
    if history.trim().is_empty() {
        println!("No update history available.");
        println!("Updates are logged to journal when the timer fires.");
    } else {
        println!("Recent update history:");
        println!("{}", history);
    }
    
    Ok(())
}
```

Add to `crates/maverick-runtime-edge/src/main.rs`:

In the `use` declarations, add:
```rust
mod update;
```

In the main match block for CLI args, add:
```rust
("update", Some(submatches)) => {
    match submatches.subcommand() {
        ("check", _) => {
            if let Err(e) = update::cli::check() {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        ("status", _) => {
            if let Err(e) = update::cli::status() {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        ("history", Some(submatches)) => {
            let n = submatches
                .value_of("LINES")
                .and_then(|v| v.parse().ok())
                .unwrap_or(10);
            
            if let Err(e) = update::cli::history(n) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        _ => {
            eprintln!("Usage: maverick-edge update [check|status|history]");
            std::process::exit(1);
        }
    }
}
```

Add clap subcommands for update (in the app setup):
```rust
Arg::new("update")
    .subcommand(App::new("check").about("Check for updates"))
    .subcommand(App::new("status").about("Show current version and update status"))
    .subcommand(App::new("history")
        .about("Show update history")
        .arg(Arg::new("LINES").default_value("10"))
    )
```
</action>

<acceptance_criteria>
- `maverick-edge update check` shows current version, mode, and available update
- `maverick-edge update status` shows version, last update time, backup count, timer status
- `maverick-edge update history` shows recent entries from journalctl
- `maverick-edge update history 5` shows last 5 entries
- All subcommands handle errors gracefully with non-zero exit
</acceptance_criteria>

<verification>
1. `cargo check -p maverick-runtime-edge` passes with update CLI
2. `maverick-edge update --help` shows available subcommands
3. `maverick-edge update check --help` shows check command help
4. Invalid subcommand shows usage error with exit code 1
5. journalctl failures handled gracefully (status shows "unknown")
</verification>

<success_criteria>
- CLI provides three operator-facing commands: check, status, history
- Commands work offline (don't require network for status/history)
- Errors are reported to stderr with exit code 1
- Help text is clear and complete
</success_criteria>

---
*Plan: 11-C*
*Created: 2026-04-17*