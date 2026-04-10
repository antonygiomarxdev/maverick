use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

const DEFAULT_BIND: &str = "0.0.0.0:17000";
const DEFAULT_READ_TIMEOUT_MS: u64 = 1_000;
const DEFAULT_MAX_MESSAGES: u32 = 1_000;

#[derive(Parser, Debug)]
#[command(name = "maverick-edge-tui")]
#[command(about = "Optional terminal UX for maverick-edge operations")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Print current local TUI configuration.
    ConfigShow,
    /// Set one or more configuration values.
    ConfigSet {
        #[arg(long)]
        data_dir: Option<String>,
        #[arg(long)]
        gwmp_bind: Option<String>,
        #[arg(long)]
        loop_read_timeout_ms: Option<u64>,
        #[arg(long)]
        loop_max_messages: Option<u32>,
    },
    /// Run `maverick-edge status` with configured env.
    Status,
    /// Run `maverick-edge health` with configured env.
    Health,
    /// Run `maverick-edge radio ingest-loop` with configured env.
    StartIngestLoop,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct TuiConfig {
    data_dir: String,
    gwmp_bind: String,
    loop_read_timeout_ms: u64,
    loop_max_messages: u32,
    enabled_extensions: Vec<String>,
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            data_dir: "data".to_string(),
            gwmp_bind: DEFAULT_BIND.to_string(),
            loop_read_timeout_ms: DEFAULT_READ_TIMEOUT_MS,
            loop_max_messages: DEFAULT_MAX_MESSAGES,
            enabled_extensions: vec!["maverick-edge-tui".to_string()],
        }
    }
}

fn main() -> Result<(), String> {
    let cli = Cli::parse();
    let mut cfg = load_or_create_config()?;

    match cli.command {
        Some(Commands::ConfigShow) => print_config(&cfg),
        Some(Commands::ConfigSet {
            data_dir,
            gwmp_bind,
            loop_read_timeout_ms,
            loop_max_messages,
        }) => {
            if let Some(v) = data_dir {
                cfg.data_dir = v;
            }
            if let Some(v) = gwmp_bind {
                cfg.gwmp_bind = v;
            }
            if let Some(v) = loop_read_timeout_ms {
                cfg.loop_read_timeout_ms = v.max(1);
            }
            if let Some(v) = loop_max_messages {
                cfg.loop_max_messages = v.max(1);
            }
            save_config(&cfg)?;
            print_config(&cfg);
        }
        Some(Commands::Status) => run_edge_command(&cfg, &["status"])?,
        Some(Commands::Health) => run_edge_command(&cfg, &["health"])?,
        Some(Commands::StartIngestLoop) => run_edge_command(
            &cfg,
            &[
                "radio",
                "ingest-loop",
                "--bind",
                &cfg.gwmp_bind,
                "--read-timeout-ms",
                &cfg.loop_read_timeout_ms.to_string(),
                "--max-messages",
                &cfg.loop_max_messages.to_string(),
            ],
        )?,
        None => interactive_menu(&mut cfg)?,
    }

    Ok(())
}

fn interactive_menu(cfg: &mut TuiConfig) -> Result<(), String> {
    loop {
        println!();
        println!("== Maverick Edge TUI ==");
        println!("1) Show config");
        println!("2) Configure essentials");
        println!("3) Status");
        println!("4) Health");
        println!("5) Start ingest-loop");
        println!("6) Quit");
        print!("Select option: ");
        io::stdout().flush().map_err(|e| e.to_string())?;
        let mut line = String::new();
        io::stdin()
            .read_line(&mut line)
            .map_err(|e| e.to_string())?;
        match line.trim() {
            "1" => print_config(cfg),
            "2" => configure_essentials(cfg)?,
            "3" => run_edge_command(cfg, &["status"])?,
            "4" => run_edge_command(cfg, &["health"])?,
            "5" => {
                let timeout = cfg.loop_read_timeout_ms.to_string();
                let max = cfg.loop_max_messages.to_string();
                run_edge_command(
                    cfg,
                    &[
                        "radio",
                        "ingest-loop",
                        "--bind",
                        &cfg.gwmp_bind,
                        "--read-timeout-ms",
                        &timeout,
                        "--max-messages",
                        &max,
                    ],
                )?;
            }
            "6" => break,
            _ => println!("Invalid option"),
        }
    }
    Ok(())
}

fn configure_essentials(cfg: &mut TuiConfig) -> Result<(), String> {
    cfg.data_dir = prompt_with_default("Data dir", &cfg.data_dir)?;
    cfg.gwmp_bind = prompt_with_default("GWMP bind", &cfg.gwmp_bind)?;
    let timeout = prompt_with_default(
        "Loop read timeout ms",
        &cfg.loop_read_timeout_ms.to_string(),
    )?;
    let max = prompt_with_default("Loop max messages", &cfg.loop_max_messages.to_string())?;
    cfg.loop_read_timeout_ms = timeout
        .parse::<u64>()
        .map_err(|e| format!("invalid timeout: {e}"))?
        .max(1);
    cfg.loop_max_messages = max
        .parse::<u32>()
        .map_err(|e| format!("invalid max messages: {e}"))?
        .max(1);
    save_config(cfg)?;
    println!("Configuration saved.");
    Ok(())
}

fn prompt_with_default(label: &str, default: &str) -> Result<String, String> {
    print!("{label} [{default}]: ");
    io::stdout().flush().map_err(|e| e.to_string())?;
    let mut line = String::new();
    io::stdin()
        .read_line(&mut line)
        .map_err(|e| e.to_string())?;
    let trimmed = line.trim();
    if trimmed.is_empty() {
        Ok(default.to_string())
    } else {
        Ok(trimmed.to_string())
    }
}

fn print_config(cfg: &TuiConfig) {
    println!(
        "{}",
        serde_json::to_string_pretty(cfg).unwrap_or_else(|_| "{}".to_string())
    );
}

fn run_edge_command(cfg: &TuiConfig, args: &[&str]) -> Result<(), String> {
    let status = Command::new("maverick-edge")
        .env("MAVERICK_DATA_DIR", &cfg.data_dir)
        .env("MAVERICK_GWMP_BIND", &cfg.gwmp_bind)
        .env(
            "MAVERICK_GWMP_LOOP_READ_TIMEOUT_MS",
            cfg.loop_read_timeout_ms.to_string(),
        )
        .env(
            "MAVERICK_GWMP_LOOP_MAX_MESSAGES",
            cfg.loop_max_messages.to_string(),
        )
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|e| format!("failed to run maverick-edge: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("maverick-edge exited with status {status}"))
    }
}

fn config_path() -> Result<PathBuf, String> {
    let base = if let Ok(v) = std::env::var("XDG_CONFIG_HOME") {
        PathBuf::from(v)
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".config")
    } else {
        return Err("cannot resolve config dir (HOME/XDG_CONFIG_HOME)".to_string());
    };
    Ok(base.join("maverick").join("tui-config.json"))
}

fn load_or_create_config() -> Result<TuiConfig, String> {
    let p = config_path()?;
    if p.exists() {
        let data =
            fs::read_to_string(&p).map_err(|e| format!("read config {}: {e}", p.display()))?;
        serde_json::from_str(&data).map_err(|e| format!("parse config {}: {e}", p.display()))
    } else {
        let cfg = TuiConfig::default();
        save_config(&cfg)?;
        Ok(cfg)
    }
}

fn save_config(cfg: &TuiConfig) -> Result<(), String> {
    let p = config_path()?;
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("create config dir {}: {e}", parent.display()))?;
    }
    let data = serde_json::to_string_pretty(cfg).map_err(|e| format!("serialize config: {e}"))?;
    fs::write(&p, data).map_err(|e| format!("write config {}: {e}", p.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_expected_values() {
        let cfg = TuiConfig::default();
        assert_eq!(cfg.gwmp_bind, DEFAULT_BIND);
        assert_eq!(cfg.loop_read_timeout_ms, DEFAULT_READ_TIMEOUT_MS);
        assert_eq!(cfg.loop_max_messages, DEFAULT_MAX_MESSAGES);
        assert_eq!(
            cfg.enabled_extensions,
            vec!["maverick-edge-tui".to_string()]
        );
    }
}
