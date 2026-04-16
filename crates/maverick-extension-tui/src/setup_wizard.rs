//! First-time setup wizard and systemd unit helpers.

use std::fs;
use std::process::{Command, Stdio};

use crate::config::{save_config, TuiConfig};
use crate::console_ui::{
    clear_screen, print_config, prompt_with_default, prompt_yes_no, SetupMode, SystemdAction,
    UiStyle,
};
use crate::doctor::probe_edge_capabilities;
use crate::edge_runner::run_edge_command;
use crate::lns_file::LNS_CONFIG_DEFAULT_PATH;
use crate::profiles::suggested_profile_from_memory;

const SYSTEMD_UNIT_PATH: &str = "/etc/systemd/system/maverick-edge.service";
const SYSTEMD_SERVICE_NAME: &str = "maverick-edge.service";

pub(crate) fn upsert_extension(extensions: &mut Vec<String>, extension: &str, enabled: bool) {
    let existing = extensions.iter().any(|e| e == extension);
    if enabled {
        if !existing {
            extensions.push(extension.to_string());
        }
    } else {
        extensions.retain(|e| e != extension);
    }
}

pub(crate) fn run_setup_non_interactive(cfg: &mut TuiConfig) -> Result<(), String> {
    upsert_extension(&mut cfg.enabled_extensions, "console", true);
    save_config(cfg)?;

    println!("Applied non-interactive setup defaults.");
    println!("Config persisted at default TUI path.");

    if let Err(error) = run_edge_command(cfg, &["status"]) {
        eprintln!("warning: status check failed after non-interactive setup: {error}");
    }
    if let Err(error) = run_edge_command(cfg, &["health"]) {
        eprintln!("warning: health check failed after non-interactive setup: {error}");
    }
    Ok(())
}

pub(crate) fn run_setup_wizard(cfg: &mut TuiConfig) -> Result<(), String> {
    let style = UiStyle::detect();

    clear_screen();
    println!(
        "{}",
        style.heading("========================================")
    );
    println!("{}", style.heading("        MAVERICK EDGE SETUP WIZARD"));
    println!(
        "{}",
        style.heading("========================================")
    );

    style.phase(1, "Welcome", SetupMode::Basic);
    println!("This setup configures Maverick for reliable edge operation.");
    println!("Estimated time: 1-3 minutes.");
    let setup_mode = prompt_setup_mode()?;

    style.phase(2, "Hardware Profile", setup_mode);
    let probe = probe_edge_capabilities();
    let mut selected_profile = if let Some(ref p) = probe {
        let suggested = suggested_profile_from_memory(p.total_memory_bytes).to_string();
        let os_name = p.os_name.clone().unwrap_or_else(|| "unknown".to_string());
        let os_version = p
            .os_version
            .clone()
            .unwrap_or_else(|| "unknown".to_string());
        println!("Detected host: {os_name} {os_version}");
        println!("Suggested profile: {suggested}");
        suggested
    } else {
        println!("Probe unavailable; default suggestion: balanced");
        "balanced".to_string()
    };

    if !prompt_yes_no("Use suggested profile", true)? {
        selected_profile = prompt_profile_choice(&selected_profile)?;
    }

    style.phase(3, "Runtime Configuration", setup_mode);
    cfg.data_dir = prompt_with_default("Data dir", &cfg.data_dir)?;
    cfg.gwmp_bind = prompt_with_default("GWMP bind", &cfg.gwmp_bind)?;
    if setup_mode == SetupMode::Advanced {
        cfg.loop_read_timeout_ms =
            prompt_positive_u64("Loop read timeout ms", cfg.loop_read_timeout_ms)?;
        cfg.loop_max_messages = prompt_loop_max_messages(cfg.loop_max_messages)?;
    } else {
        println!(
            "Basic mode keeps existing timeout={} and max_messages={}",
            cfg.loop_read_timeout_ms, cfg.loop_max_messages
        );
    }

    style.phase(4, "Integrations", setup_mode);
    let enable_tui = prompt_yes_no("Enable Maverick console extension", true)?;
    upsert_extension(&mut cfg.enabled_extensions, "console", enable_tui);
    let systemd_action = prompt_systemd_action()?;

    if systemd_action != SystemdAction::None {
        match apply_systemd_action(cfg, systemd_action) {
            Ok(message) => println!("{message}"),
            Err(error) => {
                eprintln!("warning: systemd step failed: {error}");
            }
        }
    }

    style.phase(5, "LNS declarative configuration", setup_mode);
    println!("Maverick uses {LNS_CONFIG_DEFAULT_PATH} as the operator-edited source of truth.");
    println!("Run `sudo maverick-edge config init` once to create a template, then edit devices/applications.");
    if std::path::Path::new(LNS_CONFIG_DEFAULT_PATH).exists() {
        if prompt_yes_no("Load lns-config.toml into SQLite now (config load)", true)? {
            if let Err(e) = run_edge_command(
                cfg,
                &["config", "load", "--config-path", LNS_CONFIG_DEFAULT_PATH],
            ) {
                eprintln!("warning: config load failed: {e}");
            }
        }
    } else {
        println!("File not present yet; you can initialize it after setup with:");
        println!("  sudo maverick-edge config init --config-path {LNS_CONFIG_DEFAULT_PATH}");
    }

    style.phase(6, "Verify and Commit", setup_mode);
    println!("Selected profile: {selected_profile}");
    println!("Data dir: {}", cfg.data_dir);
    println!("GWMP bind: {}", cfg.gwmp_bind);
    println!("Loop timeout ms: {}", cfg.loop_read_timeout_ms);
    println!("Loop max messages: {}", cfg.loop_max_messages);
    println!("Systemd action: {}", systemd_action.label());

    if !prompt_yes_no("Persist this configuration", true)? {
        println!("Setup cancelled before write.");
        return Ok(());
    }

    save_config(cfg)?;
    println!("Configuration saved.");

    if prompt_yes_no("Run status check now", true)? {
        if let Err(error) = run_edge_command(cfg, &["status"]) {
            eprintln!("warning: status check failed: {error}");
        }
    }
    if prompt_yes_no("Run health check now", true)? {
        if let Err(error) = run_edge_command(cfg, &["health"]) {
            eprintln!("warning: health check failed: {error}");
        }
    }

    println!();
    println!("Setup completed. Effective configuration:");
    print_config(cfg);
    Ok(())
}

pub(crate) fn configure_essentials(cfg: &mut TuiConfig) -> Result<(), String> {
    cfg.data_dir = prompt_with_default("Data dir", &cfg.data_dir)?;
    cfg.gwmp_bind = prompt_with_default("GWMP bind", &cfg.gwmp_bind)?;
    cfg.loop_read_timeout_ms =
        prompt_positive_u64("Loop read timeout ms", cfg.loop_read_timeout_ms)?;
    cfg.loop_max_messages = prompt_loop_max_messages(cfg.loop_max_messages)?;
    save_config(cfg)?;
    println!("Configuration saved.");
    Ok(())
}

fn prompt_setup_mode() -> Result<SetupMode, String> {
    println!("Select setup mode:");
    println!("1) Basic (recommended)");
    println!("2) Advanced");
    let raw = prompt_with_default("Mode", "1")?;
    match raw.trim() {
        "1" => Ok(SetupMode::Basic),
        "2" => Ok(SetupMode::Advanced),
        "q" | "Q" => Err("setup cancelled by user".to_string()),
        _ => Err("invalid mode selection (use 1 or 2)".to_string()),
    }
}

fn prompt_profile_choice(default_profile: &str) -> Result<String, String> {
    println!("Choose profile:");
    println!("1) constrained");
    println!("2) balanced");
    println!("3) high-capacity");

    let default_idx = match default_profile {
        "constrained" => "1",
        "balanced" => "2",
        "high-capacity" => "3",
        _ => "2",
    };

    let raw = prompt_with_default("Profile", default_idx)?;
    match raw.trim() {
        "1" => Ok("constrained".to_string()),
        "2" => Ok("balanced".to_string()),
        "3" => Ok("high-capacity".to_string()),
        _ => Err("invalid profile selection (use 1/2/3)".to_string()),
    }
}

fn prompt_systemd_action() -> Result<SystemdAction, String> {
    println!("Systemd options:");
    println!("1) Do not create systemd unit");
    println!("2) Create or update unit only");
    println!("3) Create or update unit and enable (no start)");

    let raw = prompt_with_default("Systemd action", "1")?;
    match raw.trim() {
        "1" => Ok(SystemdAction::None),
        "2" => Ok(SystemdAction::CreateUnit),
        "3" => Ok(SystemdAction::CreateAndEnable),
        _ => Err("invalid systemd action (use 1/2/3)".to_string()),
    }
}

fn apply_systemd_action(cfg: &TuiConfig, action: SystemdAction) -> Result<String, String> {
    fs::write(SYSTEMD_UNIT_PATH, render_systemd_unit(cfg))
        .map_err(|e| format!("write {}: {e}", SYSTEMD_UNIT_PATH))?;

    run_systemctl(&["daemon-reload"])?;

    if action == SystemdAction::CreateAndEnable {
        run_systemctl(&["enable", SYSTEMD_SERVICE_NAME])?;
    }

    Ok(format!("systemd unit updated at {}", SYSTEMD_UNIT_PATH))
}

fn run_systemctl(args: &[&str]) -> Result<(), String> {
    let status = Command::new("systemctl")
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .status()
        .map_err(|e| format!("failed to run systemctl {:?}: {e}", args))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("systemctl {:?} failed with status {status}", args))
    }
}

pub(crate) fn render_systemd_unit(cfg: &TuiConfig) -> String {
    format!(
        "[Unit]\nDescription=Maverick Edge Runtime\nAfter=network.target\n\n[Service]\nType=simple\nEnvironment=\"MAVERICK_DATA_DIR={}\"\nEnvironment=\"MAVERICK_GWMP_BIND={}\"\nEnvironment=\"MAVERICK_GWMP_LOOP_READ_TIMEOUT_MS={}\"\nEnvironment=\"MAVERICK_GWMP_LOOP_MAX_MESSAGES={}\"\nExecStart=/usr/local/bin/maverick-edge radio ingest-loop\nRestart=on-failure\nRestartSec=5\n\n[Install]\nWantedBy=multi-user.target\n",
        cfg.data_dir, cfg.gwmp_bind, cfg.loop_read_timeout_ms, cfg.loop_max_messages
    )
}

fn prompt_positive_u64(label: &str, default: u64) -> Result<u64, String> {
    let raw = prompt_with_default(label, &default.to_string())?;
    raw.parse::<u64>()
        .map_err(|e| format!("invalid {label}: {e}"))
        .map(|v| v.max(1))
}

fn prompt_loop_max_messages(default: u32) -> Result<u32, String> {
    let raw = prompt_with_default(
        "Loop max messages (0 = unlimited; recommended for systemd)",
        &default.to_string(),
    )?;
    raw.parse::<u32>()
        .map_err(|e| format!("invalid loop max messages: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TuiConfig;

    #[test]
    fn upsert_extension_adds_and_removes_expected_item() {
        let mut extensions = Vec::new();
        upsert_extension(&mut extensions, "console", true);
        assert_eq!(extensions, vec!["console".to_string()]);

        upsert_extension(&mut extensions, "console", true);
        assert_eq!(extensions, vec!["console".to_string()]);

        upsert_extension(&mut extensions, "console", false);
        assert!(extensions.is_empty());
    }

    #[test]
    fn render_systemd_unit_contains_runtime_envs() {
        let cfg = TuiConfig::default();
        let unit = render_systemd_unit(&cfg);
        assert!(unit.contains("MAVERICK_DATA_DIR=/var/lib/maverick"));
        assert!(unit.contains("MAVERICK_GWMP_BIND=0.0.0.0:17000"));
        assert!(unit.contains("ExecStart=/usr/local/bin/maverick-edge radio ingest-loop"));
    }
}
