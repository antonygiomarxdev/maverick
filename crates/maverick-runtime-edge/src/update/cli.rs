//! Update CLI commands

use crate::update::{UpdateConfig, UpdateError};

/// Run update check manually
pub fn check() -> Result<(), UpdateError> {
    let config = UpdateConfig::load()?;
    let current = UpdateConfig::current_version()?;

    println!("Current version: {}", current);
    println!(
        "Update mode: {}",
        match config.mode {
            crate::update::UpdateMode::Release => "release",
            crate::update::UpdateMode::Dev => "dev",
        }
    );

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

    let output = std::process::Command::new("journalctl")
        .args([
            "-u",
            "maverick-update.service",
            "-n",
            "1",
            "--no-pager",
            "--output=short-iso",
        ])
        .output();

    match output {
        Ok(output) if output.status.success() => {
            let last_update = String::from_utf8_lossy(&output.stdout);
            if !last_update.trim().is_empty() {
                println!("Last update: {}", last_update.trim());
            } else {
                println!("Last update: never (or no updates logged yet)");
            }
        }
        _ => {
            println!("Last update: unknown (journal unavailable)");
        }
    }

    let config = UpdateConfig::load()?;
    if let Ok(entries) = std::fs::read_dir(&config.backup_dir) {
        let count = entries
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with("maverick-edge-")
            })
            .count();
        println!("Backups available: {}", count);
    }

    let timer_output = std::process::Command::new("systemctl")
        .args(["is-active", "maverick-update.timer"])
        .output();

    if let Ok(output) = timer_output {
        let active = String::from_utf8_lossy(&output.stdout);
        println!("Update timer: {}", active.trim());
    }

    let enabled_output = std::process::Command::new("systemctl")
        .args(["is-enabled", "maverick-update.timer"])
        .output();

    if let Ok(output) = enabled_output {
        let enabled = String::from_utf8_lossy(&output.stdout);
        println!("Timer enabled: {}", enabled.trim());
    }

    Ok(())
}

/// Show update history from journal
pub fn history(n: usize) -> Result<(), UpdateError> {
    let lines = if n == 0 { 10 } else { n };

    let output = std::process::Command::new("journalctl")
        .args([
            "-u",
            "maverick-update.service",
            "-n",
            lines.to_string().as_str(),
            "--no-pager",
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
