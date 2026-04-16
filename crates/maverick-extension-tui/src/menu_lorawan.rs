//! LoRaWAN / LNS submenu: `maverick-edge config …` shortcuts and guided wizards.

use std::io::{self, Write};

use crate::config::TuiConfig;
use crate::console_ui::{clear_screen, pause_continue, UiStyle};
use crate::edge_runner::{run_edge_command, run_edge_command_or_sudo};
use crate::lns_file::LNS_CONFIG_DEFAULT_PATH;

pub(crate) fn run_lorawan_lns_menu(cfg: &TuiConfig) -> Result<(), String> {
    let style = UiStyle::detect();
    loop {
        clear_screen();
        println!(
            "{}",
            style.heading("LoRaWAN / LNS (declarative configuration)")
        );
        println!("--------------------------------------------------");
        println!("Source file (edit on the gateway): {LNS_CONFIG_DEFAULT_PATH}");
        println!("SQLite mirror uses MAVERICK_DATA_DIR: {}", cfg.data_dir);
        println!();
        println!("  [1] Show — policy + rows loaded in SQLite (`config show`)");
        println!("  [2] Validate — parse `lns-config.toml` only (`config validate`)");
        println!("  [3] List applications (JSON)");
        println!("  [4] List devices (JSON)");
        println!("  [5] List pending DevAddrs (JSON)");
        println!("  [6] Load — apply file to SQLite (`config load`; writes DB)");
        println!("  [7] Guided — applications (add/edit/remove, save, optional load)");
        println!("  [8] Guided — devices (add/edit/remove, save, optional load)");
        println!("  [9] Guided — autoprovision policy");
        println!("  [b] Back to main menu");
        print!("Select: ");
        io::stdout().flush().map_err(|e| e.to_string())?;
        let mut line = String::new();
        io::stdin()
            .read_line(&mut line)
            .map_err(|e| e.to_string())?;
        let choice = line.trim().to_ascii_lowercase();
        match choice.as_str() {
            "1" => run_edge_command(cfg, &["config", "show"])?,
            "2" => run_edge_command(
                cfg,
                &[
                    "config",
                    "validate",
                    "--config-path",
                    LNS_CONFIG_DEFAULT_PATH,
                ],
            )?,
            "3" => run_edge_command(cfg, &["config", "list-apps"])?,
            "4" => run_edge_command(cfg, &["config", "list-devices"])?,
            "5" => run_edge_command(cfg, &["config", "list-pending"])?,
            "6" => {
                print!("Run `config load` now (writes SQLite under data dir)? [y/N]: ");
                io::stdout().flush().map_err(|e| e.to_string())?;
                let mut confirm = String::new();
                io::stdin()
                    .read_line(&mut confirm)
                    .map_err(|e| e.to_string())?;
                let ok = matches!(confirm.trim().to_ascii_lowercase().as_str(), "y" | "yes");
                if ok {
                    run_edge_command_or_sudo(
                        cfg,
                        &["config", "load", "--config-path", LNS_CONFIG_DEFAULT_PATH],
                    )?;
                } else {
                    println!("Skipped config load.");
                }
            }
            "7" => {
                crate::lns_wizard::run_applications_wizard(cfg)?;
            }
            "8" => {
                crate::lns_wizard::run_devices_wizard(cfg)?;
            }
            "9" => {
                crate::lns_wizard::run_autoprovision_wizard(cfg)?;
            }
            "b" | "q" => return Ok(()),
            "" => continue,
            other => {
                println!("Unknown option: {other}");
            }
        }
        pause_continue()?;
    }
}
