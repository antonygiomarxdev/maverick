//! Monitored `radio ingest-loop` subprocess (operator console).

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

use crate::config::TuiConfig;
use crate::edge_runner::maverick_edge_command_interactive;

fn ensure_data_dir_writable_by_user(data_dir: &str) -> Result<(), String> {
    let base = Path::new(data_dir);
    if !base.is_dir() {
        return Err(format!(
            "Data directory does not exist: {data_dir}. Create it or change data_dir in config."
        ));
    }
    let probe = base.join(".maverick-write-probe");
    match OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&probe)
    {
        Ok(mut f) => {
            writeln!(f, "ok").map_err(|e| e.to_string())?;
            let _ = fs::remove_file(&probe);
            Ok(())
        }
        Err(e) => Err(format!(
            "Cannot write under {data_dir} ({e}). If this tree was created by sudo, run: sudo chown -R $(whoami) {data_dir}"
        )),
    }
}

pub(crate) fn run_ingest_loop_monitored(cfg: &TuiConfig) -> Result<(), String> {
    println!("\nStarting ingest-loop in monitored mode");
    println!("Bind: {}", cfg.gwmp_bind);
    println!("Read timeout ms: {}", cfg.loop_read_timeout_ms);
    println!("Max messages: {}", cfg.loop_max_messages);
    ensure_data_dir_writable_by_user(&cfg.data_dir)?;
    println!("Waiting for packets can look idle; heartbeat is shown every 3 seconds.");
    println!("Use Ctrl+C to stop ingest-loop.");

    let timeout = cfg.loop_read_timeout_ms.to_string();
    let max = cfg.loop_max_messages.to_string();

    let mut child = maverick_edge_command_interactive(cfg)
        .args([
            "radio",
            "ingest-loop",
            "--bind",
            &cfg.gwmp_bind,
            "--read-timeout-ms",
            &timeout,
            "--max-messages",
            &max,
        ])
        .spawn()
        .map_err(|e| format!("failed to run ingest-loop: {e}"))?;

    let started = Instant::now();
    let mut last_heartbeat = Instant::now();

    loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|e| format!("failed to poll ingest-loop status: {e}"))?
        {
            if status.success() {
                println!(
                    "\ningest-loop finished after {:?} (exit 0).",
                    started.elapsed()
                );
                return Ok(());
            }
            return Err(format!(
                "ingest-loop exited with status {status} (see JSON / stderr above for storage or bind errors)"
            ));
        }

        if last_heartbeat.elapsed() >= Duration::from_secs(3) {
            println!(
                "[heartbeat] ingest-loop running for {:?} (waiting without traffic is normal)",
                started.elapsed()
            );
            last_heartbeat = Instant::now();
        }

        thread::sleep(Duration::from_millis(400));
    }
}
