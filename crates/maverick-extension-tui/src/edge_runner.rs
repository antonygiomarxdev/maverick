//! Subprocess orchestration for the `maverick-edge` binary (operator console composition root).

use std::process::{Command, Stdio};

use crate::config::TuiConfig;

pub(crate) fn apply_maverick_env_vars(cmd: &mut Command, cfg: &TuiConfig) {
    cmd.env("MAVERICK_DATA_DIR", &cfg.data_dir)
        .env("MAVERICK_GWMP_BIND", &cfg.gwmp_bind)
        .env(
            "MAVERICK_GWMP_LOOP_READ_TIMEOUT_MS",
            cfg.loop_read_timeout_ms.to_string(),
        )
        .env(
            "MAVERICK_GWMP_LOOP_MAX_MESSAGES",
            cfg.loop_max_messages.to_string(),
        );
}

fn apply_interactive_stdio(cmd: &mut Command) {
    cmd.stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
}

/// `maverick-edge` with Maverick env vars applied; caller sets args and stdio / spawn.
pub(crate) fn maverick_edge_command(cfg: &TuiConfig) -> Command {
    let mut cmd = Command::new("maverick-edge");
    apply_maverick_env_vars(&mut cmd, cfg);
    cmd
}

pub(crate) fn maverick_edge_command_interactive(cfg: &TuiConfig) -> Command {
    let mut cmd = maverick_edge_command(cfg);
    apply_interactive_stdio(&mut cmd);
    cmd
}

pub(crate) fn maverick_edge_probe_output() -> Option<std::process::Output> {
    Command::new("maverick-edge")
        .arg("probe")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()
}

pub(crate) fn run_edge_command(cfg: &TuiConfig, args: &[&str]) -> Result<(), String> {
    let mut cmd = maverick_edge_command_interactive(cfg);
    let status = cmd
        .args(args)
        .status()
        .map_err(|e| format!("failed to run maverick-edge: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("maverick-edge exited with status {status}"))
    }
}

/// Like [`run_edge_command`], but runs `sudo -E maverick-edge …` (for `/etc`, SQLite under root, etc.).
pub(crate) fn run_edge_command_sudo(cfg: &TuiConfig, args: &[&str]) -> Result<(), String> {
    let mut cmd = Command::new("sudo");
    cmd.arg("-E").arg("maverick-edge");
    apply_maverick_env_vars(&mut cmd, cfg);
    apply_interactive_stdio(&mut cmd);
    let status = cmd
        .args(args)
        .status()
        .map_err(|e| format!("failed to run sudo maverick-edge: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("sudo maverick-edge exited with status {status}"))
    }
}

/// Runs [`run_edge_command`], then on failure on Unix retries with [`run_edge_command_sudo`].
pub(crate) fn run_edge_command_or_sudo(cfg: &TuiConfig, args: &[&str]) -> Result<(), String> {
    match run_edge_command(cfg, args) {
        Ok(()) => Ok(()),
        Err(first) => {
            if cfg!(unix) {
                println!("Trying again with sudo (needed for paths under /etc or system data)…");
                run_edge_command_sudo(cfg, args).map_err(|second| format!("{first}\n{second}"))
            } else {
                Err(first)
            }
        }
    }
}

pub(crate) fn run_edge_json_command(
    cfg: &TuiConfig,
    args: &[&str],
) -> Result<serde_json::Value, String> {
    let mut cmd = maverick_edge_command(cfg);
    let output = cmd
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("failed to run maverick-edge {:?}: {e}", args))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "maverick-edge {:?} failed with status {}: {}",
            args,
            output.status,
            stderr.trim()
        ));
    }

    serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("failed to parse json from maverick-edge {:?}: {e}", args))
}
