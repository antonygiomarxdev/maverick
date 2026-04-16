//! Maverick console binary: composition root for operator UX (menus, setup, subprocess to `maverick-edge`).

mod config;
mod console_ui;
mod doctor;
mod edge_runner;
mod ingest_loop;
mod lns_file;
mod lns_wizard;
mod menu_interactive;
mod menu_lorawan;
mod profiles;
mod setup_wizard;

use clap::{Parser, Subcommand};

use crate::config::{load_or_create_config, save_config};
use crate::console_ui::print_config;
use crate::doctor::{probe_edge_capabilities, run_doctor_dashboard};
use crate::edge_runner::run_edge_command;
use crate::menu_interactive::interactive_menu;
use crate::profiles::apply_profile_by_name;
use crate::setup_wizard::{run_setup_non_interactive, run_setup_wizard};

fn warn_deprecated_invocation_name() {
    let Some(arg0) = std::env::args().next() else {
        return;
    };
    let base = std::path::Path::new(&arg0)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    if base == "maverick-edge-tui" {
        eprintln!(
            "Notice: command 'maverick-edge-tui' is deprecated; prefer 'maverick' (same binary)."
        );
    }
}

#[derive(Parser, Debug)]
#[command(name = "maverick")]
#[command(visible_alias = "maverick-edge-tui")]
#[command(about = "Optional Maverick console (terminal UX) for maverick-edge operations")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Run guided first-time setup wizard.
    Setup {
        /// Apply defaults without interactive prompts.
        #[arg(long)]
        non_interactive: bool,
    },
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
    /// Run a consolidated operator diagnosis and recommendations.
    Doctor,
    /// Apply profile defaults quickly (auto/constrained/balanced/high-capacity).
    ApplyProfile {
        #[arg(long, default_value = "auto")]
        profile: String,
    },
    /// Run `maverick-edge radio ingest-loop` with configured env.
    StartIngestLoop,
}

fn main() -> Result<(), String> {
    warn_deprecated_invocation_name();
    let cli = Cli::parse();
    let mut cfg = load_or_create_config()?;

    match cli.command {
        Some(Commands::Setup { non_interactive }) => {
            if non_interactive {
                run_setup_non_interactive(&mut cfg)?;
            } else {
                run_setup_wizard(&mut cfg)?;
            }
        }
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
                cfg.loop_max_messages = v;
            }
            save_config(&cfg)?;
            print_config(&cfg);
        }
        Some(Commands::Status) => run_edge_command(&cfg, &["status"])?,
        Some(Commands::Health) => run_edge_command(&cfg, &["health"])?,
        Some(Commands::Doctor) => run_doctor_dashboard(&cfg)?,
        Some(Commands::ApplyProfile { profile }) => {
            apply_profile_by_name(
                &mut cfg,
                &profile,
                probe_edge_capabilities().map(|p| p.total_memory_bytes),
            )?;
            save_config(&cfg)?;
            println!("Profile '{profile}' applied and configuration saved.");
            print_config(&cfg);
        }
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

#[cfg(test)]
mod tests {
    use crate::config::{TuiConfig, DEFAULT_BIND, DEFAULT_MAX_MESSAGES, DEFAULT_READ_TIMEOUT_MS};

    #[test]
    fn default_config_has_expected_values() {
        let cfg = TuiConfig::default();
        assert_eq!(cfg.gwmp_bind, DEFAULT_BIND);
        assert_eq!(cfg.loop_read_timeout_ms, DEFAULT_READ_TIMEOUT_MS);
        assert_eq!(cfg.loop_max_messages, DEFAULT_MAX_MESSAGES);
        assert_eq!(cfg.enabled_extensions, vec!["console".to_string()]);
    }
}
