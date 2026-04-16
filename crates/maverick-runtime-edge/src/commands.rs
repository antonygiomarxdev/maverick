//! CLI command handlers (keeps `main` thin).

pub mod config;

use std::io::{ErrorKind, IsTerminal};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;

use maverick_adapter_persistence_sqlite::{SqlitePersistence, SqlitePersistenceOptions};
use maverick_adapter_radio_udp::{ResiliencePolicy, ResilientRadioTransport, UdpDownlinkTransport};
use maverick_core::health::{ComponentHealth, HealthState, HealthStatus};
use maverick_core::ports::{DownlinkFrame, RadioTransport};
use maverick_core::{InstallProfile, StoragePressureLevel, StoragePressureSource};
use maverick_domain::{identifiers::Eui64, DevAddr, GatewayEui};
use serde_json::json;

use crate::cli_constants::{
    DEFAULT_GWMP_BIND_ADDR, DEFAULT_LNS_CONFIG_PATH, HEALTH_COMPONENT_RADIO_ENVIRONMENT,
    HEALTH_COMPONENT_STORAGE, RADIO_PROBE_PAYLOAD_BYTE, RECENT_ERRORS_NOT_WIRED_MESSAGE,
    STORAGE_OPEN_FAILED_PREFIX,
};
use crate::edge_json::{self, RadioProbeOutcome, RecentErrorsStubResponse};
use crate::paths::db_path;
use crate::probe::{health_from_probe, total_disk_bytes_hint, HardwareCapabilities};
use crate::runtime_capabilities::RuntimeCapabilityReport;

fn storage_level_to_health(level: StoragePressureLevel) -> HealthStatus {
    match level {
        StoragePressureLevel::Normal => HealthStatus::Healthy,
        StoragePressureLevel::Elevated
        | StoragePressureLevel::Critical
        | StoragePressureLevel::HardLimit => HealthStatus::Degraded,
    }
}

fn sqlite_opts() -> SqlitePersistenceOptions {
    SqlitePersistenceOptions {
        total_disk_bytes: total_disk_bytes_hint(),
        ..SqlitePersistenceOptions::default()
    }
}

fn gwmp_bind_effective() -> String {
    std::env::var("MAVERICK_GWMP_BIND").unwrap_or_else(|_| DEFAULT_GWMP_BIND_ADDR.to_string())
}

pub(crate) fn run_setup(non_interactive: bool) {
    if !non_interactive && (!std::io::stdin().is_terminal() || !std::io::stdout().is_terminal()) {
        eprintln!(
            "setup requires an interactive terminal; run maverick-edge setup directly from a TTY"
        );
        std::process::exit(2);
    }

    let preferred =
        std::env::var("MAVERICK_CONSOLE_BIN").unwrap_or_else(|_| "maverick".to_string());
    let mut command = Command::new(&preferred);
    command.arg("setup");
    if non_interactive {
        command.arg("--non-interactive");
    }

    let status = match command
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
    {
        Ok(status) => status,
        Err(error) if error.kind() == ErrorKind::NotFound && preferred != "maverick-edge-tui" => {
            let mut legacy = Command::new("maverick-edge-tui");
            legacy.arg("setup");
            if non_interactive {
                legacy.arg("--non-interactive");
            }
            match legacy
                .stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .status()
            {
                Ok(status) => status,
                Err(err) => {
                    eprintln!(
                        "failed to launch Maverick console ({preferred} / maverick-edge-tui): {err}. Install the console extension and ensure it is in PATH"
                    );
                    std::process::exit(1);
                }
            }
        }
        Err(error) => {
            eprintln!(
                "failed to launch Maverick console ({preferred}): {error}. Ensure the console extension is installed and in PATH"
            );
            std::process::exit(1);
        }
    };

    if !status.success() {
        if let Some(code) = status.code() {
            std::process::exit(code);
        }
        std::process::exit(1);
    }
}

pub(crate) async fn run_status(data_dir: PathBuf, db_file: &str) {
    let report = RuntimeCapabilityReport::build(
        gwmp_bind_effective(),
        Some(Path::new(DEFAULT_LNS_CONFIG_PATH)),
    );
    let cap = report.hardware.clone();
    let dbp = db_path(&data_dir, db_file);
    let storage = if dbp.exists() {
        let profile = cap.suggested_install_profile();
        let policy = profile.default_storage_policy();
        let opts = sqlite_opts();
        match SqlitePersistence::open(&dbp, policy, opts) {
            Ok(store) => {
                let snap = store.pressure_snapshot().await;
                edge_json::storage_present_ok(&snap)
            }
            Err(e) => edge_json::storage_present_err(&e),
        }
    } else {
        edge_json::storage_absent()
    };
    let runtime_capabilities =
        serde_json::to_value(&report).unwrap_or_else(|e| json!({ "error": e.to_string() }));
    let doc = edge_json::status_document(
        &data_dir,
        format!("{:?}", cap.suggested_install_profile()),
        cap.total_memory_bytes,
        storage,
        runtime_capabilities,
    );
    println!("{}", serde_json::to_string(&doc).expect("status json"));
}

pub(crate) async fn run_health(data_dir: PathBuf, db_file: &str) {
    let report = RuntimeCapabilityReport::build(
        gwmp_bind_effective(),
        Some(Path::new(DEFAULT_LNS_CONFIG_PATH)),
    );
    let cap = report.hardware.clone();
    let mut components = health_from_probe(&cap).components;
    let radio_detail = format!(
        "ingest={} bind={} snapshot_ms={} forwarder_hints={}",
        report.selected_ingest.backend_id,
        report.selected_ingest.listen_bind,
        report.capability_snapshot.snapshot_id_ms,
        report
            .radio_environment
            .packet_forwarder_service_hints
            .len(),
    );
    let radio_status = if report
        .radio_environment
        .packet_forwarder_service_hints
        .is_empty()
        && cfg!(target_os = "linux")
    {
        HealthStatus::Degraded
    } else {
        HealthStatus::Healthy
    };
    components.push(ComponentHealth {
        name: HEALTH_COMPONENT_RADIO_ENVIRONMENT.to_string(),
        status: radio_status,
        detail: Some(radio_detail),
    });
    let dbp = db_path(&data_dir, db_file);
    if dbp.exists() {
        let profile = cap.suggested_install_profile();
        let policy = profile.default_storage_policy();
        let opts = sqlite_opts();
        match SqlitePersistence::open(&dbp, policy, opts) {
            Ok(store) => {
                let snap = store.pressure_snapshot().await;
                components.push(ComponentHealth {
                    name: HEALTH_COMPONENT_STORAGE.to_string(),
                    status: storage_level_to_health(snap.level),
                    detail: snap.detail,
                });
            }
            Err(e) => {
                components.push(ComponentHealth {
                    name: HEALTH_COMPONENT_STORAGE.to_string(),
                    status: HealthStatus::Unhealthy,
                    detail: Some(format!("{STORAGE_OPEN_FAILED_PREFIX}{e}")),
                });
            }
        }
    }
    let h = HealthState::new(components);
    println!("{}", serde_json::to_string_pretty(&h).expect("health json"));
}

pub(crate) fn run_recent_errors(lines: usize) {
    let body = RecentErrorsStubResponse {
        message: RECENT_ERRORS_NOT_WIRED_MESSAGE,
        lines_requested: lines,
    };
    println!(
        "{}",
        serde_json::to_string(&body).expect("recent-errors json")
    );
}

pub(crate) fn run_probe(summary: bool) {
    let report = RuntimeCapabilityReport::build(
        gwmp_bind_effective(),
        Some(Path::new(DEFAULT_LNS_CONFIG_PATH)),
    );
    if summary {
        print!("{}", report.format_operator_summary());
    } else {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).expect("probe json")
        );
    }
}

pub(crate) fn run_storage_policy(profile: InstallProfile) {
    let pol = profile.default_storage_policy();
    println!(
        "{}",
        serde_json::to_string_pretty(&pol).expect("policy json")
    );
}

pub(crate) async fn run_storage_pressure(data_dir: PathBuf, db_file: &str) {
    let dbp = db_path(&data_dir, db_file);
    if !dbp.exists() {
        println!(
            "{}",
            serde_json::to_string(&edge_json::storage_pressure_absent(&data_dir))
                .expect("pressure absent json")
        );
        return;
    }
    let cap = HardwareCapabilities::probe();
    let profile = cap.suggested_install_profile();
    let policy = profile.default_storage_policy();
    let opts = sqlite_opts();
    match SqlitePersistence::open(&dbp, policy, opts) {
        Ok(store) => {
            let snap = store.pressure_snapshot().await;
            println!(
                "{}",
                serde_json::to_string_pretty(&snap).expect("snap json")
            );
        }
        Err(e) => {
            println!(
                "{}",
                serde_json::to_string(&edge_json::storage_pressure_open_err(&e))
                    .expect("pressure err json")
            );
        }
    }
}

/// Sends one UDP datagram using the resilient transport wrapper; failures are JSON-only (no panic).
pub(crate) async fn run_radio_downlink_probe(host: String, port: u16) {
    let gateway: SocketAddr = match format!("{host}:{port}").parse() {
        Ok(a) => a,
        Err(e) => {
            let v = edge_json::radio_probe_result(
                RadioProbeOutcome::Failed,
                &host,
                port,
                0,
                Some(format!("invalid gateway address: {e}")),
            );
            println!("{}", serde_json::to_string(&v).expect("probe json"));
            return;
        }
    };

    let payload = vec![RADIO_PROBE_PAYLOAD_BYTE];
    let frame = DownlinkFrame {
        gateway_eui: GatewayEui(Eui64([0; 8])),
        dev_addr: DevAddr(0),
        payload: payload.clone(),
    };

    let udp = match UdpDownlinkTransport::bind_ephemeral(gateway).await {
        Ok(u) => u,
        Err(e) => {
            let v = edge_json::radio_probe_result(
                RadioProbeOutcome::Failed,
                &host,
                port,
                payload.len(),
                Some(e.to_string()),
            );
            println!("{}", serde_json::to_string(&v).expect("probe json"));
            return;
        }
    };

    let inner: Arc<dyn RadioTransport> = Arc::new(udp);
    let resilient = ResilientRadioTransport::new(inner, ResiliencePolicy::default());

    match resilient.send_downlink(&frame).await {
        Ok(()) => {
            let v = edge_json::radio_probe_result(
                RadioProbeOutcome::Sent,
                &host,
                port,
                payload.len(),
                None,
            );
            println!("{}", serde_json::to_string(&v).expect("probe json"));
        }
        Err(e) => {
            let v = edge_json::radio_probe_result(
                RadioProbeOutcome::Failed,
                &host,
                port,
                payload.len(),
                Some(e.to_string()),
            );
            println!("{}", serde_json::to_string(&v).expect("probe json"));
        }
    }
}
