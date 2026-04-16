//! Declarative LNS configuration (file `/etc/maverick/lns-config.toml` is source of truth).
//!
//! `schema_version = 1` is the **initial released** document shape: explicit **OTAA vs ABP** (`activation_mode`),
//! optional `dev_addr` for OTAA, and required `dev_addr` for ABP.

use serde::{Deserialize, Serialize};

/// Root document for `lns-config.toml`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LnsConfigDocument {
    pub schema_version: u32,
    #[serde(default)]
    pub autoprovision: AutoprovisionPolicy,
    /// Optional radio ingest backend. When omitted, behavior matches pre–Phase-2 configs (UDP GWMP).
    #[serde(default)]
    pub radio: Option<RadioConfig>,
    #[serde(default)]
    pub applications: Vec<ApplicationEntry>,
    #[serde(default)]
    pub devices: Vec<DeviceEntry>,
}

impl Default for LnsConfigDocument {
    fn default() -> Self {
        Self {
            schema_version: Self::CURRENT_SCHEMA_VERSION,
            autoprovision: AutoprovisionPolicy::default(),
            radio: None,
            applications: Vec::new(),
            devices: Vec::new(),
        }
    }
}

/// Ingest path: Semtech GWMP/UDP (default) or direct SPI concentrator (Phase 2+).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RadioBackend {
    Udp,
    Spi,
}

/// Optional `[radio]` table in `lns-config.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RadioConfig {
    pub backend: RadioBackend,
    /// SPI device path when `backend = spi` (e.g. `/dev/spidev0.0`).
    #[serde(default)]
    pub spi_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AutoprovisionPolicy {
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Max new pending rows per gateway per minute (sliding window enforced in runtime).
    #[serde(default = "default_rate_limit")]
    pub rate_limit_per_gateway_per_minute: u32,
    /// Pending rows older than this (seconds) can be pruned by operator tooling.
    #[serde(default = "default_pending_ttl")]
    pub pending_ttl_secs: u64,
}

fn default_true() -> bool {
    true
}

fn default_rate_limit() -> u32 {
    10
}

fn default_pending_ttl() -> u64 {
    86_400
}

impl Default for AutoprovisionPolicy {
    fn default() -> Self {
        Self {
            enabled: true,
            rate_limit_per_gateway_per_minute: default_rate_limit(),
            pending_ttl_secs: default_pending_ttl(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApplicationEntry {
    pub id: String,
    #[serde(default)]
    pub name: String,
    pub default_region: String,
}

/// How the device is activated on air (ChirpStack-style split).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ActivationMode {
    Otaa,
    Abp,
}

/// One device row in `lns-config.toml`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeviceEntry {
    pub activation_mode: ActivationMode,
    /// 16 hex chars (64-bit EUI), optional `0x` prefix allowed in parser.
    pub dev_eui: String,
    /// 8 hex chars (32-bit DevAddr), optional `0x` prefix. Required for **ABP**; omit or leave empty for **OTAA** until a session exists.
    #[serde(default)]
    pub dev_addr: Option<String>,
    pub application_id: String,
    pub region: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// OTAA key material (required when `activation_mode` is `Otaa`).
    #[serde(default)]
    pub otaa: Option<OtaaKeys>,
    /// Optional ABP session keys (hex); not required for ingest until downlink/crypto is wired.
    #[serde(default)]
    pub abp: Option<AbpKeys>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OtaaKeys {
    /// 16 hex chars JoinEUI.
    pub join_eui: String,
    /// 32 hex chars (128-bit AppKey).
    pub app_key: String,
    /// 32 hex chars (128-bit NwkKey); optional for 1.0.x single key.
    #[serde(default)]
    pub nwk_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct AbpKeys {
    /// 32 hex chars AppSKey (optional in file for minimal ABP).
    #[serde(default)]
    pub apps_key: Option<String>,
    /// 32 hex chars NwkSKey (optional).
    #[serde(default)]
    pub nwks_key: Option<String>,
}

impl LnsConfigDocument {
    pub const CURRENT_SCHEMA_VERSION: u32 = 1;

    /// Validate IDs and hex fields; does not open the database.
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != Self::CURRENT_SCHEMA_VERSION {
            return Err(format!(
                "unsupported schema_version {} (expected {})",
                self.schema_version,
                Self::CURRENT_SCHEMA_VERSION
            ));
        }
        if let Some(ref radio) = self.radio {
            match radio.backend {
                RadioBackend::Spi => {
                    let path = radio.spi_path.as_deref().unwrap_or("").trim();
                    if path.is_empty() {
                        return Err(
                            "radio.backend = spi requires non-empty radio.spi_path (e.g. /dev/spidev0.0)"
                                .to_string(),
                        );
                    }
                }
                RadioBackend::Udp => {}
            }
        }
        for app in &self.applications {
            if app.id.is_empty() {
                return Err("application id must not be empty".to_string());
            }
            if app.default_region.trim().is_empty() {
                return Err(format!("application {}: default_region required", app.id));
            }
            app.default_region
                .parse::<maverick_domain::RegionId>()
                .map_err(|e| format!("application {}: invalid default_region: {e}", app.id))?;
        }
        let app_ids: std::collections::HashSet<_> =
            self.applications.iter().map(|a| a.id.as_str()).collect();
        for d in &self.devices {
            if d.application_id.is_empty() {
                return Err("device: application_id required".to_string());
            }
            if !app_ids.contains(d.application_id.as_str()) {
                return Err(format!(
                    "device {}: unknown application_id {}",
                    d.dev_eui, d.application_id
                ));
            }
            parse_hex_dev_eui(&d.dev_eui).map_err(|e| format!("device dev_eui: {e}"))?;
            d.region
                .parse::<maverick_domain::RegionId>()
                .map_err(|e| format!("device {}: invalid region: {e}", d.dev_eui))?;

            match d.activation_mode {
                ActivationMode::Otaa => {
                    let Some(ref k) = d.otaa else {
                        return Err(format!(
                            "device {}: OTAA requires [devices.otaa] join_eui and app_key",
                            d.dev_eui
                        ));
                    };
                    parse_hex_16(&k.join_eui).map_err(|e| format!("otaa.join_eui: {e}"))?;
                    parse_hex_32(&k.app_key).map_err(|e| format!("otaa.app_key: {e}"))?;
                    if let Some(ref nk) = k.nwk_key {
                        parse_hex_32(nk).map_err(|e| format!("otaa.nwk_key: {e}"))?;
                    }
                    if let Some(ref addr) = d.dev_addr {
                        if !addr.trim().is_empty() {
                            parse_hex_dev_addr(addr)
                                .map_err(|e| format!("device dev_addr: {e}"))?;
                        }
                    }
                    if d.abp.is_some() {
                        return Err(format!(
                            "device {}: ABP keys block must not be set for OTAA devices",
                            d.dev_eui
                        ));
                    }
                }
                ActivationMode::Abp => {
                    let addr = d.dev_addr.as_ref().ok_or_else(|| {
                        format!("device {}: ABP requires dev_addr (8 hex)", d.dev_eui)
                    })?;
                    if addr.trim().is_empty() {
                        return Err(format!(
                            "device {}: ABP requires dev_addr (8 hex)",
                            d.dev_eui
                        ));
                    }
                    parse_hex_dev_addr(addr).map_err(|e| format!("device dev_addr: {e}"))?;
                    if let Some(ref abp) = d.abp {
                        if let Some(ref s) = abp.apps_key {
                            if !s.trim().is_empty() {
                                parse_hex_32(s).map_err(|e| format!("abp.apps_key: {e}"))?;
                            }
                        }
                        if let Some(ref s) = abp.nwks_key {
                            if !s.trim().is_empty() {
                                parse_hex_32(s).map_err(|e| format!("abp.nwks_key: {e}"))?;
                            }
                        }
                    }
                    if d.otaa.is_some() {
                        return Err(format!(
                            "device {}: OTAA block must not be set for ABP devices",
                            d.dev_eui
                        ));
                    }
                }
            }
        }
        Ok(())
    }
}

fn strip_hex_prefix(s: &str) -> &str {
    s.trim().strip_prefix("0x").unwrap_or(s.trim())
}

/// Parse 16 hex chars -> 8 bytes (DevEUI).
pub fn parse_hex_dev_eui(s: &str) -> Result<[u8; 8], String> {
    let h = strip_hex_prefix(s);
    if h.len() != 16 {
        return Err(format!("expected 16 hex chars, got {}", h.len()));
    }
    parse_hex_bytes_fixed::<8>(h)
}

/// Parse 8 hex chars -> 4 bytes (DevAddr stored as u32 elsewhere).
pub fn parse_hex_dev_addr(s: &str) -> Result<u32, String> {
    let h = strip_hex_prefix(s);
    if h.len() != 8 {
        return Err(format!("expected 8 hex chars for DevAddr, got {}", h.len()));
    }
    let b = parse_hex_bytes_fixed::<4>(h)?;
    Ok(u32::from_be_bytes(b))
}

pub fn parse_hex_16(s: &str) -> Result<[u8; 8], String> {
    let h = strip_hex_prefix(s);
    if h.len() != 16 {
        return Err(format!("expected 16 hex chars, got {}", h.len()));
    }
    parse_hex_bytes_fixed::<8>(h)
}

pub fn parse_hex_32(s: &str) -> Result<[u8; 16], String> {
    let h = strip_hex_prefix(s);
    if h.len() != 32 {
        return Err(format!("expected 32 hex chars, got {}", h.len()));
    }
    parse_hex_bytes_fixed::<16>(h)
}

fn parse_hex_bytes_fixed<const N: usize>(hex: &str) -> Result<[u8; N], String> {
    if hex.len() != N * 2 {
        return Err("hex length mismatch".to_string());
    }
    let mut out = [0u8; N];
    for i in 0..N {
        let byte = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16)
            .map_err(|_| format!("invalid hex at offset {}", i * 2))?;
        out[i] = byte;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_app() -> ApplicationEntry {
        ApplicationEntry {
            id: "app1".to_string(),
            name: "Test".to_string(),
            default_region: "EU868".to_string(),
        }
    }

    #[test]
    fn validates_abp_device() {
        let doc = LnsConfigDocument {
            schema_version: 1,
            autoprovision: AutoprovisionPolicy::default(),
            radio: None,
            applications: vec![sample_app()],
            devices: vec![DeviceEntry {
                activation_mode: ActivationMode::Abp,
                dev_eui: "0102030405060708".to_string(),
                dev_addr: Some("01ABCDEF".to_string()),
                application_id: "app1".to_string(),
                region: "EU868".to_string(),
                enabled: true,
                otaa: None,
                abp: None,
            }],
        };
        doc.validate().expect("valid abp");
    }

    #[test]
    fn validates_otaa_without_dev_addr() {
        let doc = LnsConfigDocument {
            schema_version: 1,
            autoprovision: AutoprovisionPolicy::default(),
            radio: None,
            applications: vec![sample_app()],
            devices: vec![DeviceEntry {
                activation_mode: ActivationMode::Otaa,
                dev_eui: "0102030405060708".to_string(),
                dev_addr: None,
                application_id: "app1".to_string(),
                region: "EU868".to_string(),
                enabled: true,
                otaa: Some(OtaaKeys {
                    join_eui: "0000000000000000".to_string(),
                    app_key: "00000000000000000000000000000000".to_string(),
                    nwk_key: None,
                }),
                abp: None,
            }],
        };
        doc.validate().expect("valid otaa no dev_addr");
    }

    #[test]
    fn rejects_abp_without_dev_addr() {
        let doc = LnsConfigDocument {
            schema_version: 1,
            autoprovision: AutoprovisionPolicy::default(),
            radio: None,
            applications: vec![sample_app()],
            devices: vec![DeviceEntry {
                activation_mode: ActivationMode::Abp,
                dev_eui: "0102030405060708".to_string(),
                dev_addr: None,
                application_id: "app1".to_string(),
                region: "EU868".to_string(),
                enabled: true,
                otaa: None,
                abp: None,
            }],
        };
        assert!(doc.validate().is_err());
    }

    #[test]
    fn rejects_otaa_without_keys() {
        let doc = LnsConfigDocument {
            schema_version: 1,
            autoprovision: AutoprovisionPolicy::default(),
            radio: None,
            applications: vec![sample_app()],
            devices: vec![DeviceEntry {
                activation_mode: ActivationMode::Otaa,
                dev_eui: "0102030405060708".to_string(),
                dev_addr: None,
                application_id: "app1".to_string(),
                region: "EU868".to_string(),
                enabled: true,
                otaa: None,
                abp: None,
            }],
        };
        assert!(doc.validate().is_err());
    }

    #[test]
    fn rejects_unsupported_schema_version() {
        let doc = LnsConfigDocument {
            schema_version: 0,
            autoprovision: AutoprovisionPolicy::default(),
            radio: None,
            applications: vec![sample_app()],
            devices: vec![],
        };
        assert!(doc.validate().is_err());
    }

    #[test]
    fn rejects_spi_backend_without_spi_path() {
        let doc = LnsConfigDocument {
            schema_version: 1,
            autoprovision: AutoprovisionPolicy::default(),
            radio: Some(RadioConfig {
                backend: RadioBackend::Spi,
                spi_path: None,
            }),
            applications: vec![sample_app()],
            devices: vec![],
        };
        assert!(doc.validate().is_err());
    }

    #[test]
    fn accepts_spi_with_spi_path() {
        let doc = LnsConfigDocument {
            schema_version: 1,
            autoprovision: AutoprovisionPolicy::default(),
            radio: Some(RadioConfig {
                backend: RadioBackend::Spi,
                spi_path: Some("/dev/spidev0.0".to_string()),
            }),
            applications: vec![sample_app()],
            devices: vec![],
        };
        doc.validate().expect("spi path set");
    }
}
