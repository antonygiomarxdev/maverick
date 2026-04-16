//! Shared configuration loading for the Maverick console extension.
//!
//! Precedence for runtime fields used with `maverick-edge` (later layers override earlier):
//! 1. Built-in defaults
//! 2. `/etc/maverick/runtime.env`
//! 3. `/etc/maverick/setup.json` (`runtime` block)
//! 4. `~/.config/maverick/tui-config.json`

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub(crate) const DEFAULT_BIND: &str = "0.0.0.0:17000";
pub(crate) const DEFAULT_READ_TIMEOUT_MS: u64 = 1_000;
/// `0` matches `maverick-edge` default: unlimited UDP receive iterations (systemd-friendly).
pub(crate) const DEFAULT_MAX_MESSAGES: u32 = 0;

pub(crate) const RUNTIME_ENV_PATH: &str = "/etc/maverick/runtime.env";
pub(crate) const SETUP_JSON_PATH: &str = "/etc/maverick/setup.json";

/// Non-critical console preferences (optional).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub(crate) struct ConsolePrefs {
    pub(crate) schema_version: u32,
    pub(crate) theme: String,
}

impl Default for ConsolePrefs {
    fn default() -> Self {
        Self {
            schema_version: 1,
            theme: "auto".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct SetupRuntimeBlock {
    pub(crate) data_dir: String,
    pub(crate) gwmp_bind: String,
    pub(crate) loop_read_timeout_ms: u64,
    pub(crate) loop_max_messages: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct SetupState {
    pub(crate) schema_version: u32,
    pub(crate) completed_at: Option<String>,
    pub(crate) installer_version: String,
    #[serde(default)]
    pub(crate) selected_extensions: serde_json::Value,
    pub(crate) runtime: SetupRuntimeBlock,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct TuiConfig {
    pub(crate) data_dir: String,
    pub(crate) gwmp_bind: String,
    pub(crate) loop_read_timeout_ms: u64,
    pub(crate) loop_max_messages: u32,
    pub(crate) enabled_extensions: Vec<String>,
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            data_dir: "/var/lib/maverick".to_string(),
            gwmp_bind: DEFAULT_BIND.to_string(),
            loop_read_timeout_ms: DEFAULT_READ_TIMEOUT_MS,
            loop_max_messages: DEFAULT_MAX_MESSAGES,
            enabled_extensions: vec!["console".to_string()],
        }
    }
}

pub(crate) fn console_prefs_path() -> Result<PathBuf, String> {
    let base = if let Ok(v) = std::env::var("XDG_CONFIG_HOME") {
        PathBuf::from(v)
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".config")
    } else {
        return Err("cannot resolve config dir (HOME/XDG_CONFIG_HOME)".to_string());
    };
    Ok(base.join("maverick").join("console.toml"))
}

pub(crate) fn tui_config_json_path() -> Result<PathBuf, String> {
    let base = if let Ok(v) = std::env::var("XDG_CONFIG_HOME") {
        PathBuf::from(v)
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".config")
    } else {
        return Err("cannot resolve config dir (HOME/XDG_CONFIG_HOME)".to_string());
    };
    Ok(base.join("maverick").join("tui-config.json"))
}

pub(crate) fn load_or_create_config() -> Result<TuiConfig, String> {
    let mut cfg = TuiConfig::default();
    apply_runtime_env_overlay(&mut cfg, Path::new(RUNTIME_ENV_PATH))?;
    apply_setup_json_overlay(&mut cfg, Path::new(SETUP_JSON_PATH))?;

    let p = tui_config_json_path()?;
    if p.exists() {
        let data =
            fs::read_to_string(&p).map_err(|e| format!("read config {}: {e}", p.display()))?;
        let user_cfg: TuiConfig = serde_json::from_str(&data)
            .map_err(|e| format!("parse config {}: {e}", p.display()))?;
        cfg = merge_user_config(cfg, user_cfg);
    } else {
        save_config(&cfg)?;
    }
    Ok(cfg)
}

fn merge_user_config(mut base: TuiConfig, user: TuiConfig) -> TuiConfig {
    base.data_dir = user.data_dir;
    base.gwmp_bind = user.gwmp_bind;
    base.loop_read_timeout_ms = user.loop_read_timeout_ms;
    base.loop_max_messages = user.loop_max_messages;
    base.enabled_extensions = normalize_extension_ids(&user.enabled_extensions);
    base
}

fn normalize_extension_ids(ext: &[String]) -> Vec<String> {
    ext.iter()
        .map(|e| {
            if e == "maverick-edge-tui" {
                "console".to_string()
            } else {
                e.clone()
            }
        })
        .collect()
}

pub(crate) fn save_config(cfg: &TuiConfig) -> Result<(), String> {
    let p = tui_config_json_path()?;
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("create config dir {}: {e}", parent.display()))?;
    }
    let data = serde_json::to_string_pretty(cfg).map_err(|e| format!("serialize config: {e}"))?;
    fs::write(&p, data).map_err(|e| format!("write config {}: {e}", p.display()))?;

    ensure_console_prefs_stub()?;
    Ok(())
}

fn ensure_console_prefs_stub() -> Result<(), String> {
    let p = console_prefs_path()?;
    if p.exists() {
        return Ok(());
    }
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("create console prefs dir {}: {e}", parent.display()))?;
    }
    let prefs = ConsolePrefs::default();
    let toml_data =
        toml::to_string_pretty(&prefs).map_err(|e| format!("serialize console.toml: {e}"))?;
    fs::write(&p, toml_data).map_err(|e| format!("write {}: {e}", p.display()))
}

pub(crate) fn load_console_prefs() -> ConsolePrefs {
    let p = match console_prefs_path() {
        Ok(p) => p,
        Err(_) => return ConsolePrefs::default(),
    };
    if !p.exists() {
        return ConsolePrefs::default();
    }
    let data = match fs::read_to_string(&p) {
        Ok(d) => d,
        Err(_) => return ConsolePrefs::default(),
    };
    toml::from_str(&data).unwrap_or_default()
}

fn apply_runtime_env_overlay(cfg: &mut TuiConfig, path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    let data = fs::read_to_string(path)
        .map_err(|e| format!("read runtime env {}: {e}", path.display()))?;
    for line in data.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            let key = k.trim();
            let val = v.trim().trim_matches('"');
            match key {
                "MAVERICK_DATA_DIR" => cfg.data_dir = val.to_string(),
                "MAVERICK_GWMP_BIND" => cfg.gwmp_bind = val.to_string(),
                "MAVERICK_GWMP_LOOP_READ_TIMEOUT_MS" => {
                    if let Ok(n) = val.parse::<u64>() {
                        cfg.loop_read_timeout_ms = n.max(1);
                    }
                }
                "MAVERICK_GWMP_LOOP_MAX_MESSAGES" => {
                    if let Ok(n) = val.parse::<u32>() {
                        cfg.loop_max_messages = n;
                    }
                }
                _ => {}
            }
        }
    }
    Ok(())
}

fn apply_setup_json_overlay(cfg: &mut TuiConfig, path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    let data = fs::read_to_string(path)
        .map_err(|e| format!("read setup state {}: {e}", path.display()))?;
    let setup: SetupState =
        serde_json::from_str(&data).map_err(|e| format!("parse setup state: {e}"))?;
    sync_extensions_from_setup(cfg, &setup);
    cfg.data_dir = setup.runtime.data_dir;
    cfg.gwmp_bind = setup.runtime.gwmp_bind;
    cfg.loop_read_timeout_ms = setup.runtime.loop_read_timeout_ms.max(1);
    cfg.loop_max_messages = setup.runtime.loop_max_messages;
    Ok(())
}

fn sync_extensions_from_setup(cfg: &mut TuiConfig, setup: &SetupState) {
    if let Some(console) = setup.selected_extensions.get("console") {
        if let Some(state) = console.as_str() {
            let enable = state == "enabled" || state == "installed";
            upsert_extension_id(&mut cfg.enabled_extensions, "console", enable);
        }
    }
}

fn upsert_extension_id(extensions: &mut Vec<String>, extension: &str, enabled: bool) {
    let existing = extensions.iter().any(|e| e == extension);
    if enabled {
        if !existing {
            extensions.push(extension.to_string());
        }
    } else {
        extensions.retain(|e| e != extension);
    }
}

pub(crate) fn onboarding_completed_hint() -> bool {
    let Ok(data) = fs::read_to_string(SETUP_JSON_PATH) else {
        return false;
    };
    let Ok(setup) = serde_json::from_str::<SetupState>(&data) else {
        return false;
    };
    setup.completed_at.is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_env_overlay_parses_keys() {
        let mut cfg = TuiConfig::default();
        let path = std::env::temp_dir().join("maverick_runtime_env_test.env");
        std::fs::write(
            &path,
            r#"MAVERICK_DATA_DIR=/data
MAVERICK_GWMP_BIND=127.0.0.1:17000
MAVERICK_GWMP_LOOP_READ_TIMEOUT_MS=2000
MAVERICK_GWMP_LOOP_MAX_MESSAGES=0
"#,
        )
        .unwrap();
        apply_runtime_env_overlay(&mut cfg, &path).unwrap();
        let _ = std::fs::remove_file(&path);
        assert_eq!(cfg.data_dir, "/data");
        assert_eq!(cfg.gwmp_bind, "127.0.0.1:17000");
        assert_eq!(cfg.loop_read_timeout_ms, 2000);
        assert_eq!(cfg.loop_max_messages, 0);
    }
}
