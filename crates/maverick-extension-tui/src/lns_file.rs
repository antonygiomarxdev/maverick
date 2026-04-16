//! Load/save `lns-config.toml` using core schema (`maverick-core`).

use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use maverick_core::lns_config::LnsConfigDocument;

/// Canonical path on Linux gateways (matches `maverick-edge` default).
pub const LNS_CONFIG_DEFAULT_PATH: &str = "/etc/maverick/lns-config.toml";

/// Load document from disk, or return an empty schema-v1 document if the file is missing.
pub fn load_or_default(path: &Path) -> Result<(LnsConfigDocument, bool), String> {
    if !path.exists() {
        return Ok((LnsConfigDocument::default(), false));
    }
    let data = fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let doc: LnsConfigDocument =
        toml::from_str(&data).map_err(|e| format!("parse TOML {}: {e}", path.display()))?;
    Ok((doc, true))
}

/// Validate, serialize, and write. On permission denied under `/etc`, write a temp file and return instructions.
pub enum SaveLnsOutcome {
    /// Written to the requested path.
    Ok,
    /// Written to a temp path; operator must copy with sudo.
    WroteTemp {
        temp_path: std::path::PathBuf,
        target: std::path::PathBuf,
    },
}

pub fn save_lns_document(path: &Path, doc: &LnsConfigDocument) -> Result<SaveLnsOutcome, String> {
    doc.validate()?;
    let s =
        toml::to_string_pretty(doc).map_err(|e| format!("serialize lns-config to TOML: {e}"))?;
    match fs::write(path, &s) {
        Ok(()) => Ok(SaveLnsOutcome::Ok),
        Err(e) if e.kind() == ErrorKind::PermissionDenied => {
            let temp_path = std::env::temp_dir().join("maverick-lns-config.toml");
            fs::write(&temp_path, &s)
                .map_err(|e| format!("write temp {}: {e}", temp_path.display()))?;
            Ok(SaveLnsOutcome::WroteTemp {
                temp_path,
                target: path.to_path_buf(),
            })
        }
        Err(e) => Err(format!("write {}: {e}", path.display())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maverick_core::lns_config::{
        ActivationMode, ApplicationEntry, AutoprovisionPolicy, DeviceEntry,
    };

    #[test]
    fn roundtrip_minimal_doc_tmpfile() {
        let path = std::env::temp_dir().join(format!(
            "maverick-lns-roundtrip-{}.toml",
            std::process::id()
        ));
        let doc = LnsConfigDocument {
            schema_version: 1,
            autoprovision: AutoprovisionPolicy::default(),
            radio: None,
            applications: vec![ApplicationEntry {
                id: "a1".to_string(),
                name: "A".to_string(),
                default_region: "EU868".to_string(),
            }],
            devices: vec![DeviceEntry {
                activation_mode: ActivationMode::Abp,
                dev_eui: "0102030405060708".to_string(),
                dev_addr: Some("01ABCDEF".to_string()),
                application_id: "a1".to_string(),
                region: "EU868".to_string(),
                enabled: true,
                otaa: None,
                abp: None,
            }],
        };
        assert!(matches!(
            save_lns_document(&path, &doc).unwrap(),
            SaveLnsOutcome::Ok
        ));
        let (loaded, existed) = load_or_default(&path).unwrap();
        assert!(existed);
        assert_eq!(loaded, doc);
        let _ = fs::remove_file(&path);
    }
}
