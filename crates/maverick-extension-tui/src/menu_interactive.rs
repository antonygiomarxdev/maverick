//! Main interactive console loop and overview.

use std::io::{self, Write};

use crate::config::{load_console_prefs, onboarding_completed_hint, save_config, TuiConfig};
use crate::console_ui::{clear_screen, console_clock, pause_continue, print_config, UiStyle};
use crate::doctor::{probe_edge_capabilities, run_doctor_dashboard};
use crate::edge_runner::{run_edge_command, run_edge_json_command};
use crate::ingest_loop::run_ingest_loop_monitored;
use crate::menu_lorawan::run_lorawan_lns_menu;
use crate::profiles::apply_suggested_profile;
use crate::setup_wizard::{configure_essentials, run_setup_wizard};

pub(crate) fn interactive_menu(cfg: &mut TuiConfig) -> Result<(), String> {
    let style = UiStyle::detect();
    let prefs = load_console_prefs();
    let mut last_event = String::from("ready");
    loop {
        clear_screen();
        println!("{}", style.heading("MAVERICK CONSOLE"));
        println!("---------------------------------");
        if onboarding_completed_hint() {
            println!("Onboarding: complete (see /etc/maverick/setup.json)");
        } else {
            println!(
                "Onboarding: not completed — run the Linux installer wizard or `maverick setup`"
            );
        }
        println!(
            "Prefs: theme={}  |  Time: {}  |  Last event: {}",
            prefs.theme,
            console_clock(),
            last_event
        );
        println!("\nCore Views");
        println!("  [o] Overview dashboard");
        println!("  [d] Doctor recommendations");
        println!("  [s] Runtime status snapshot");
        println!("  [h] Runtime health snapshot");
        println!("\nOperations");
        println!("  [i] Start ingest-loop (monitored)");
        println!("  [p] Apply suggested profile");
        println!("  [e] Configure essentials");
        println!("  [l] LoRaWAN / LNS (devices, apps, validate, load)");
        println!("  [w] Setup wizard");
        println!("  [c] Show raw config");
        println!("\nSession");
        println!("  [q] Quit");
        print!("Select action: ");
        io::stdout().flush().map_err(|e| e.to_string())?;
        let mut line = String::new();
        io::stdin()
            .read_line(&mut line)
            .map_err(|e| e.to_string())?;
        let action = line.trim().to_ascii_lowercase();
        match action.as_str() {
            "1" | "o" => {
                last_event = run_overview_dashboard(cfg)
                    .map(|_| "ok: overview refreshed".to_string())
                    .unwrap_or_else(|e| format!("error: overview failed ({e})"));
                pause_continue()?;
            }
            "2" | "e" => {
                last_event = configure_essentials(cfg)
                    .map(|_| "ok: essentials updated".to_string())
                    .unwrap_or_else(|e| format!("error: configure failed ({e})"));
                pause_continue()?;
            }
            "3" | "d" => {
                last_event = run_doctor_dashboard(cfg)
                    .map(|_| "ok: doctor completed".to_string())
                    .unwrap_or_else(|e| format!("error: doctor failed ({e})"));
                pause_continue()?;
            }
            "4" | "s" => {
                last_event = run_edge_command(cfg, &["status"])
                    .map(|_| "ok: status fetched".to_string())
                    .unwrap_or_else(|e| format!("error: status failed ({e})"));
                pause_continue()?;
            }
            "5" | "h" => {
                last_event = run_edge_command(cfg, &["health"])
                    .map(|_| "ok: health fetched".to_string())
                    .unwrap_or_else(|e| format!("error: health failed ({e})"));
                pause_continue()?;
            }
            "6" | "p" => {
                apply_suggested_profile(
                    cfg,
                    probe_edge_capabilities().map(|p| p.total_memory_bytes),
                );
                last_event = save_config(cfg)
                    .map(|_| "ok: suggested profile applied".to_string())
                    .unwrap_or_else(|e| format!("error: profile apply failed ({e})"));
                print_config(cfg);
                pause_continue()?;
            }
            "7" | "i" => {
                last_event = run_ingest_loop_monitored(cfg)
                    .map(|_| "ok: ingest-loop completed".to_string())
                    .unwrap_or_else(|e| format!("error: ingest-loop failed ({e})"));
                pause_continue()?;
            }
            "l" => {
                last_event = run_lorawan_lns_menu(cfg)
                    .map(|_| "ok: LoRaWAN / LNS menu closed".to_string())
                    .unwrap_or_else(|e| format!("error: LoRaWAN / LNS menu ({e})"));
                pause_continue()?;
            }
            "8" | "w" => {
                last_event = run_setup_wizard(cfg)
                    .map(|_| "ok: setup wizard completed".to_string())
                    .unwrap_or_else(|e| format!("error: setup wizard failed ({e})"));
                pause_continue()?;
            }
            "c" => {
                print_config(cfg);
                last_event = "ok: raw config shown".to_string();
                pause_continue()?;
            }
            "9" | "q" => break,
            _ => {
                last_event = format!("error: unknown action '{}'", action);
            }
        }
    }
    Ok(())
}

fn run_overview_dashboard(cfg: &TuiConfig) -> Result<(), String> {
    let style = UiStyle::detect();
    println!("\n{}", style.heading("== Overview =="));

    let probe = probe_edge_capabilities();
    let status = run_edge_json_command(cfg, &["status"])?;
    let health = run_edge_json_command(cfg, &["health"])?;

    if let Some(p) = probe {
        println!(
            "Host: {} {} | Memory: {} bytes",
            p.os_name.clone().unwrap_or_else(|| "unknown".to_string()),
            p.os_version
                .clone()
                .unwrap_or_else(|| "unknown".to_string()),
            p.total_memory_bytes
        );
    }

    let suggested = status
        .get("suggested_profile")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let data_dir = status
        .get("data_dir")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let storage_present = status
        .get("storage")
        .and_then(|v| v.get("present"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let overall = health
        .get("overall")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    println!("Profile: {suggested}");
    println!("Data dir: {data_dir}");
    println!(
        "Storage: {}",
        if storage_present { "present" } else { "absent" }
    );
    println!("Health: {overall}");
    println!("GWMP bind: {}", cfg.gwmp_bind);
    println!(
        "Loop policy: read_timeout_ms={} max_messages={}",
        cfg.loop_read_timeout_ms, cfg.loop_max_messages
    );

    Ok(())
}
