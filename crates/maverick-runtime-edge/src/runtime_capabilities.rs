//! Startup-time capability detection: radio environment hints + selected ingest mode (GWMP UDP today).
//!
//! Expensive work belongs here (or `reload`), not on the uplink hot path.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use maverick_core::ports::UplinkBackendKind;
use serde::Serialize;

use crate::cli_constants::DEFAULT_LNS_CONFIG_PATH;
use crate::probe::HardwareCapabilities;
use crate::radio_ingest_selection::{resolve_radio_ingest, RadioIngestSelection};

/// Stable wire snapshot for correlating logs / support bundles (recomputed on startup / config reload).
#[derive(Debug, Clone, Serialize)]
pub struct CapabilitySnapshot {
    pub snapshot_version: u32,
    /// Monotonic-ish identifier for this build of the snapshot (unix time ms).
    pub snapshot_id_ms: u128,
    pub backend_kind: UplinkBackendKind,
    pub backend_id: &'static str,
    pub listen_bind: String,
    pub lns_config_path: String,
    pub lns_config_mtime_unix_secs: Option<u64>,
}

/// Best-effort signals about the host radio stack (no HAT-specific claims without evidence).
#[derive(Debug, Clone, Serialize)]
pub struct RadioEnvironmentHints {
    pub platform: &'static str,
    /// `/run/systemd/system` exists (Linux systemd hosts).
    pub systemd_runtime_present: bool,
    /// Heuristic matches from `systemctl` (may be empty on non-Linux or minimal images).
    pub packet_forwarder_service_hints: Vec<String>,
    /// Actionable notes for operators (never silent failures to infer environment).
    pub notes: Vec<String>,
}

/// Full JSON report for `probe`, embedded in `status`, and summarized in `health`.
#[derive(Debug, Clone, Serialize)]
pub struct RuntimeCapabilityReport {
    pub hardware: HardwareCapabilities,
    pub radio_environment: RadioEnvironmentHints,
    pub selected_ingest: SelectedIngestMode,
    pub capability_snapshot: CapabilitySnapshot,
}

#[derive(Debug, Clone, Serialize)]
pub struct SelectedIngestMode {
    pub kind: UplinkBackendKind,
    pub backend_id: &'static str,
    pub listen_bind: String,
}

impl RuntimeCapabilityReport {
    /// Plain-text summary for operators (TTY / `--summary`); JSON remains the machine contract.
    pub fn format_operator_summary(&self) -> String {
        use std::fmt::Write as _;
        let mut s = String::new();
        let _ = writeln!(
            s,
            "Maverick runtime capabilities (human summary — use `probe` without `--summary` for JSON)"
        );
        let _ = writeln!(s);
        let _ = writeln!(
            s,
            "  Host: {} {}",
            self.hardware.os_name.as_deref().unwrap_or("unknown OS"),
            self.hardware.os_version.as_deref().unwrap_or("")
        );
        let _ = writeln!(
            s,
            "  Memory: {} bytes (suggested profile from probe: {:?})",
            self.hardware.total_memory_bytes,
            self.hardware.suggested_install_profile()
        );
        let _ = writeln!(s);
        let _ = writeln!(
            s,
            "  Selected uplink ingest: {} ({:?})",
            self.selected_ingest.backend_id, self.selected_ingest.kind
        );
        if self.selected_ingest.kind == UplinkBackendKind::GwmpUdp {
            let _ = writeln!(
                s,
                "  GWMP listen bind: {}  (env MAVERICK_GWMP_BIND overrides default)",
                self.selected_ingest.listen_bind
            );
        } else {
            let _ = writeln!(
                s,
                "  SPI concentrator device: {}  (from lns-config [radio], libloragw integration pending)",
                self.selected_ingest.listen_bind
            );
        }
        let _ = writeln!(s);
        let lns = std::path::Path::new(&self.capability_snapshot.lns_config_path);
        let lns_state = if lns.is_file() {
            format!(
                "present (mtime unix secs: {:?})",
                self.capability_snapshot.lns_config_mtime_unix_secs
            )
        } else {
            "not found on disk yet".to_string()
        };
        let _ = writeln!(
            s,
            "  Declarative LNS file: {} — {}",
            self.capability_snapshot.lns_config_path, lns_state
        );
        let _ = writeln!(s);
        let _ = writeln!(s, "  Radio environment:");
        let _ = writeln!(
            s,
            "    platform={}  systemd_runtime_present={}",
            self.radio_environment.platform, self.radio_environment.systemd_runtime_present
        );
        let n = self.radio_environment.packet_forwarder_service_hints.len();
        let _ = writeln!(
            s,
            "    packet_forwarder_service_hints: {} unit(s) matched heuristics",
            n
        );
        for u in self
            .radio_environment
            .packet_forwarder_service_hints
            .iter()
            .take(12)
        {
            let _ = writeln!(s, "      - {u}");
        }
        if n > 12 {
            let _ = writeln!(s, "      …");
        }
        let _ = writeln!(s);
        let _ = writeln!(s, "  Confirm / next steps:");
        if self.selected_ingest.kind == UplinkBackendKind::GwmpUdp {
            let _ = writeln!(
                s,
                "    - Confirm your Semtech packet forwarder sends GWMP PUSH_DATA to UDP {}.",
                self.selected_ingest.listen_bind
            );
        } else {
            let _ = writeln!(
                s,
                "    - SPI direct mode: ensure concentrator matches {} and libloragw is integrated.",
                self.selected_ingest.listen_bind
            );
        }
        if self
            .capability_snapshot
            .lns_config_mtime_unix_secs
            .is_none()
            && lns.is_file()
        {
            let _ = writeln!(
                s,
                "    - LNS file exists but mtime could not be read; check permissions."
            );
        }
        if !lns.is_file() {
            let _ = writeln!(
                s,
                "    - Initialize LNS: `maverick-edge config init` then edit, then `config load`."
            );
        }
        for note in &self.radio_environment.notes {
            let _ = writeln!(s, "    - {note}");
        }
        let _ = writeln!(s);
        let _ = writeln!(
            s,
            "  Snapshot id: {} (correlate with ingest startup logs)",
            self.capability_snapshot.snapshot_id_ms
        );
        s
    }

    /// Build a fresh report using `lns-config.toml` (if present) plus CLI GWMP bind default.
    pub fn build(gwmp_bind: String, lns_config_path: Option<&Path>) -> Self {
        let hardware = HardwareCapabilities::probe();
        let radio_environment = RadioEnvironmentHints::probe();
        let lns_path = lns_config_path
            .map(Path::to_path_buf)
            .unwrap_or_else(|| Path::new(DEFAULT_LNS_CONFIG_PATH).to_path_buf());
        let selection = resolve_radio_ingest(&lns_path, gwmp_bind.clone()).unwrap_or_else(|_| {
            RadioIngestSelection::Udp {
                bind: gwmp_bind.clone(),
            }
        });
        let (backend_kind, backend_id, listen_bind) = match &selection {
            RadioIngestSelection::Udp { bind } => {
                (UplinkBackendKind::GwmpUdp, "gwmp_udp", bind.clone())
            }
            RadioIngestSelection::Spi { spi_path } => (
                UplinkBackendKind::ConcentratorSpi,
                "sx130x_spi",
                spi_path.clone(),
            ),
        };
        let lns_config_mtime_unix_secs = file_mtime_secs(&lns_path);
        let snapshot_id_ms = unix_time_ms();
        let capability_snapshot = CapabilitySnapshot {
            snapshot_version: 1,
            snapshot_id_ms,
            backend_kind,
            backend_id,
            listen_bind: listen_bind.clone(),
            lns_config_path: lns_path.display().to_string(),
            lns_config_mtime_unix_secs,
        };
        let selected_ingest = SelectedIngestMode {
            kind: backend_kind,
            backend_id,
            listen_bind,
        };
        Self {
            hardware,
            radio_environment,
            selected_ingest,
            capability_snapshot,
        }
    }
}

impl RadioEnvironmentHints {
    fn probe() -> Self {
        let platform = current_platform_label();
        let systemd_runtime_present = Path::new("/run/systemd/system").exists();
        let mut notes = Vec::new();
        let packet_forwarder_service_hints = if cfg!(target_os = "linux") {
            probe_linux_forwarder_hints(&mut notes)
        } else {
            notes.push(
                "Packet-forwarder service scan skipped (non-Linux host); GWMP UDP remains available."
                    .to_string(),
            );
            Vec::new()
        };
        if packet_forwarder_service_hints.is_empty() && cfg!(target_os = "linux") {
            notes.push(
                "No common packet-forwarder units matched heuristics; confirm your forwarder targets the GWMP bind."
                    .to_string(),
            );
        }
        Self {
            platform,
            systemd_runtime_present,
            packet_forwarder_service_hints,
            notes,
        }
    }
}

/// Emit startup tracing for an ingest worker from an already-built [`RuntimeCapabilityReport`].
pub fn log_ingest_capability_report(report: &RuntimeCapabilityReport) {
    tracing::info!(
        snapshot_version = report.capability_snapshot.snapshot_version,
        snapshot_id_ms = report.capability_snapshot.snapshot_id_ms,
        backend_id = report.capability_snapshot.backend_id,
        listen_bind = %report.capability_snapshot.listen_bind,
        lns_config_revision = ?report.capability_snapshot.lns_config_mtime_unix_secs,
        "ingest capability snapshot (startup)"
    );
}

fn unix_time_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

fn file_mtime_secs(path: &Path) -> Option<u64> {
    std::fs::metadata(path)
        .ok()?
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs())
}

fn current_platform_label() -> &'static str {
    if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "unknown"
    }
}

#[cfg(target_os = "linux")]
fn probe_linux_forwarder_hints(notes: &mut Vec<String>) -> Vec<String> {
    let systemctl = which_systemctl();
    let Some(systemctl) = systemctl else {
        notes.push("`systemctl` not found; cannot enumerate LoRa forwarder units.".to_string());
        return Vec::new();
    };
    let output = match std::process::Command::new(systemctl)
        .args([
            "list-units",
            "--type=service",
            "--all",
            "--no-pager",
            "--no-legend",
        ])
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            notes.push(format!("systemctl enumeration failed: {e}"));
            return Vec::new();
        }
    };
    if !output.status.success() {
        notes.push("systemctl exited non-zero while listing units".to_string());
        return Vec::new();
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let mut hints = Vec::new();
    for line in text.lines().take(512) {
        let unit = line.split_whitespace().next().unwrap_or("").trim();
        if unit.is_empty() {
            continue;
        }
        let lower = unit.to_ascii_lowercase();
        if lower.contains("packet")
            || lower.contains("forward")
            || lower.contains("sx130")
            || lower.contains("concentrat")
            || lower.contains("loragw")
        {
            hints.push(unit.to_string());
        }
        if hints.len() >= 32 {
            break;
        }
    }
    hints.sort();
    hints.dedup();
    hints
}

#[cfg(not(target_os = "linux"))]
fn probe_linux_forwarder_hints(_notes: &mut Vec<String>) -> Vec<String> {
    Vec::new()
}

#[cfg(target_os = "linux")]
fn which_systemctl() -> Option<&'static str> {
    if Path::new("/usr/bin/systemctl").exists() {
        Some("/usr/bin/systemctl")
    } else if Path::new("/bin/systemctl").exists() {
        Some("/bin/systemctl")
    } else {
        None
    }
}
