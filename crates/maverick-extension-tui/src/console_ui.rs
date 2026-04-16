//! Terminal styling and shared prompts for the Maverick console.

use std::io::{self, Write};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config::TuiConfig;

pub(crate) const TOTAL_SETUP_PHASES: usize = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SetupMode {
    Basic,
    Advanced,
}

impl SetupMode {
    pub(crate) fn label(self) -> &'static str {
        match self {
            SetupMode::Basic => "Basic",
            SetupMode::Advanced => "Advanced",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SystemdAction {
    None,
    CreateUnit,
    CreateAndEnable,
}

impl SystemdAction {
    pub(crate) fn label(self) -> &'static str {
        match self {
            SystemdAction::None => "Do not create systemd unit",
            SystemdAction::CreateUnit => "Create or update unit",
            SystemdAction::CreateAndEnable => "Create or update unit and enable",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct UiStyle {
    color: bool,
}

impl UiStyle {
    pub(crate) fn detect() -> Self {
        let no_color = std::env::var("NO_COLOR").is_ok();
        let term_dumb = std::env::var("TERM")
            .map(|v| v.eq_ignore_ascii_case("dumb"))
            .unwrap_or(false);
        Self {
            color: !no_color && !term_dumb,
        }
    }

    pub(crate) fn heading(self, text: &str) -> String {
        if self.color {
            format!("\x1b[1;36m{text}\x1b[0m")
        } else {
            text.to_string()
        }
    }

    pub(crate) fn phase(self, idx: usize, title: &str, mode: SetupMode) {
        println!();
        println!(
            "{}",
            self.heading(&format!(
                "Phase {idx}/{TOTAL_SETUP_PHASES} [{mode}] - {title}",
                mode = mode.label()
            ))
        );
        println!("Actions: [Enter] continue  [q] cancel");
    }
}

pub(crate) fn clear_screen() {
    print!("\x1B[2J\x1B[H");
    let _ = io::stdout().flush();
}

pub(crate) fn console_clock() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let sec = now % 60;
    let min = (now / 60) % 60;
    let hour = (now / 3600) % 24;
    format!("{hour:02}:{min:02}:{sec:02}")
}

pub(crate) fn pause_continue() -> Result<(), String> {
    print!("\nPress Enter to continue...");
    io::stdout().flush().map_err(|e| e.to_string())?;
    let mut line = String::new();
    io::stdin()
        .read_line(&mut line)
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub(crate) fn prompt_yes_no(label: &str, default_yes: bool) -> Result<bool, String> {
    let default_hint = if default_yes { "Y/n" } else { "y/N" };
    print!("{label}? [{default_hint}]: ");
    io::stdout().flush().map_err(|e| e.to_string())?;

    let mut line = String::new();
    io::stdin()
        .read_line(&mut line)
        .map_err(|e| e.to_string())?;

    let answer = line.trim().to_ascii_lowercase();
    if answer.is_empty() {
        return Ok(default_yes);
    }

    match answer.as_str() {
        "q" => Err("setup cancelled by user".to_string()),
        "y" | "yes" => Ok(true),
        "n" | "no" => Ok(false),
        _ => Err(format!("invalid answer for {label}: {answer}")),
    }
}

pub(crate) fn prompt_with_default(label: &str, default: &str) -> Result<String, String> {
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

pub(crate) fn print_config(cfg: &TuiConfig) {
    println!(
        "{}",
        serde_json::to_string_pretty(cfg).unwrap_or_else(|_| "{}".to_string())
    );
}
