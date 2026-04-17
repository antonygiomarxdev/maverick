//! Maverick auto-update mechanism
//!
//! Supports two modes:
//! - `release`: Download pre-built binary from release URL
//! - `dev`: Build from source via git pull + cargo build

pub mod cli;
pub mod download;
pub mod version;

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateConfig {
    pub mode: UpdateMode,
    pub release_url: Option<String>,
    pub check_interval: u64,
    pub download_dir: PathBuf,
    pub backup_dir: PathBuf,
    pub insecure: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UpdateMode {
    Release,
    Dev,
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            mode: UpdateMode::Release,
            release_url: None,
            check_interval: 3600,
            download_dir: PathBuf::from("/var/lib/maverick/downloads"),
            backup_dir: PathBuf::from("/var/lib/maverick/backups"),
            insecure: false,
        }
    }
}

impl UpdateConfig {
    /// Load config from /etc/maverick/maverick.toml
    pub fn load() -> Result<Self, UpdateError> {
        let config_path = Path::new("/etc/maverick/maverick.toml");
        if !config_path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(config_path)
            .map_err(|e| UpdateError::ConfigRead(e.to_string()))?;

        Self::parse_toml(&content)
    }

    fn parse_toml(content: &str) -> Result<Self, UpdateError> {
        let mut config = Self::default();

        let mut in_update_section = false;
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with('[') {
                in_update_section = line == "[update]";
                continue;
            }
            if !in_update_section {
                continue;
            }

            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim().trim_matches('"');

                match key {
                    "mode" => {
                        config.mode = match value {
                            "dev" => UpdateMode::Dev,
                            _ => UpdateMode::Release,
                        };
                    }
                    "release_url" => {
                        if !value.is_empty() {
                            config.release_url = Some(value.to_string());
                        }
                    }
                    "check_interval" => {
                        if let Ok(interval) = value.parse() {
                            config.check_interval = interval;
                        }
                    }
                    "download_dir" => {
                        if !value.is_empty() {
                            config.download_dir = PathBuf::from(value);
                        }
                    }
                    "backup_dir" => {
                        if !value.is_empty() {
                            config.backup_dir = PathBuf::from(value);
                        }
                    }
                    "insecure" => {
                        config.insecure = value == "true";
                    }
                    _ => {}
                }
            }
        }

        Ok(config)
    }

    /// Get current installed version
    pub fn current_version() -> Result<String, UpdateError> {
        let output = Command::new("/usr/local/bin/maverick-edge")
            .arg("--version")
            .output()
            .map_err(|e| UpdateError::Command(e.to_string()))?;

        let version_str = String::from_utf8_lossy(&output.stdout);
        let version = version_str
            .split_whitespace()
            .nth(1)
            .unwrap_or("unknown")
            .to_string();

        Ok(version)
    }

    /// Check for available update (release mode)
    pub fn check_release_update(&self) -> Result<Option<String>, UpdateError> {
        let release_url = self
            .release_url
            .as_ref()
            .ok_or_else(|| UpdateError::Config("release_url not set".to_string()))?;

        let arch = std::env::consts::ARCH;
        let version_url = format!("{}/{}/version.txt", release_url.trim_end_matches('/'), arch);

        let new_version = version::fetch_remote_version(&version_url, self.insecure)?;

        let current = Self::current_version()?;
        if new_version > current {
            Ok(Some(new_version))
        } else {
            Ok(None)
        }
    }

    /// Download and apply update (release mode)
    pub fn apply_release_update(&self, new_version: &str) -> Result<(), UpdateError> {
        let release_url = self
            .release_url
            .as_ref()
            .ok_or_else(|| UpdateError::Config("release_url not set".to_string()))?;

        let arch = std::env::consts::ARCH;
        let binary_url = format!(
            "{}/{}/maverick-edge-{}",
            release_url.trim_end_matches('/'),
            arch,
            new_version
        );

        std::fs::create_dir_all(&self.download_dir).map_err(|e| UpdateError::Io(e.to_string()))?;
        std::fs::create_dir_all(&self.backup_dir).map_err(|e| UpdateError::Io(e.to_string()))?;

        let download_path = self
            .download_dir
            .join(format!("maverick-edge-{}", new_version));
        download::download_file(&binary_url, &download_path, self.insecure)?;

        let binary_path = Path::new("/usr/local/bin/maverick-edge");
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let backup_path = self.backup_dir.join(format!(
            "maverick-edge-{}-{}",
            Self::current_version().unwrap_or_else(|_| "unknown".to_string()),
            timestamp
        ));

        std::fs::copy(binary_path, &backup_path)
            .map_err(|e| UpdateError::Io(format!("backup failed: {}", e)))?;

        let new_binary_path = self.download_dir.join("maverick-edge-new");
        std::fs::copy(&download_path, &new_binary_path)
            .map_err(|e| UpdateError::Io(format!("copy to .new failed: {}", e)))?;

        std::fs::rename(&new_binary_path, binary_path)
            .map_err(|e| UpdateError::Io(format!("atomic replace failed: {}", e)))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(binary_path)
                .map_err(|e| UpdateError::Io(e.to_string()))?
                .permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(binary_path, perms)
                .map_err(|e| UpdateError::Io(e.to_string()))?;
        }

        let _ = std::fs::remove_file(&download_path);

        self.cleanup_old_backups()?;

        Ok(())
    }

    /// Check for dev update (git pull + build)
    pub fn check_dev_update(&self, repo_path: &Path) -> Result<Option<String>, UpdateError> {
        let output = Command::new("git")
            .args(["fetch", "--tags"])
            .current_dir(repo_path)
            .output()
            .map_err(|e| UpdateError::Command(e.to_string()))?;

        if !output.status.success() {
            return Err(UpdateError::Git("git fetch failed".to_string()));
        }

        let output = Command::new("git")
            .args(["describe", "--tags"])
            .current_dir(repo_path)
            .output()
            .map_err(|e| UpdateError::Command(e.to_string()))?;

        if !output.status.success() {
            return Ok(None);
        }

        let remote_version = String::from_utf8_lossy(&output.stdout).trim().to_string();

        let current = Self::current_version()?;
        if remote_version != current {
            Ok(Some(remote_version))
        } else {
            Ok(None)
        }
    }

    /// Apply dev update (git pull + build)
    pub fn apply_dev_update(&self, repo_path: &Path) -> Result<(), UpdateError> {
        let output = Command::new("git")
            .arg("pull")
            .current_dir(repo_path)
            .output()
            .map_err(|e| UpdateError::Command(e.to_string()))?;

        if !output.status.success() {
            return Err(UpdateError::Git("git pull failed".to_string()));
        }

        let output = Command::new("cargo")
            .args(["build", "--release", "--manifest-path", "Cargo.toml"])
            .current_dir(repo_path)
            .output()
            .map_err(|e| UpdateError::Command(e.to_string()))?;

        if !output.status.success() {
            return Err(UpdateError::Build(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let binary_path = Path::new("/usr/local/bin/maverick-edge");
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let backup_path = self
            .backup_dir
            .join(format!("maverick-edge-dev-{}", timestamp));

        std::fs::create_dir_all(&self.backup_dir).map_err(|e| UpdateError::Io(e.to_string()))?;

        std::fs::copy(binary_path, &backup_path)
            .map_err(|e| UpdateError::Io(format!("backup failed: {}", e)))?;

        let new_binary = repo_path.join("target/release/maverick-edge");
        std::fs::copy(&new_binary, binary_path)
            .map_err(|e| UpdateError::Io(format!("replace failed: {}", e)))?;

        self.cleanup_old_backups()?;

        Ok(())
    }

    /// Cleanup old backups (keep last 2)
    fn cleanup_old_backups(&self) -> Result<(), UpdateError> {
        let entries: Vec<_> = std::fs::read_dir(&self.backup_dir)
            .map_err(|e| UpdateError::Io(e.to_string()))?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with("maverick-edge-")
            })
            .collect();

        if entries.len() <= 2 {
            return Ok(());
        }

        let mut sorted: Vec<_> = entries;
        sorted.sort_by_key(|e| e.metadata().and_then(|m| m.modified()).ok());

        for entry in sorted.iter().take(sorted.len() - 2) {
            let _ = std::fs::remove_file(entry.path());
        }

        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum UpdateError {
    #[error("Config error: {0}")]
    Config(String),
    #[error("Config read: {0}")]
    ConfigRead(String),
    #[error("IO error: {0}")]
    Io(String),
    #[error("Command error: {0}")]
    Command(String),
    #[error("Git error: {0}")]
    Git(String),
    #[error("Build error: {0}")]
    Build(String),
    #[error("Download error: {0}")]
    Download(String),
    #[error("Version check error: {0}")]
    Version(String),
}
