//! CLI command handlers (keeps `main` thin).

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use maverick_adapter_persistence_sqlite::{SqlitePersistence, SqlitePersistenceOptions};
use maverick_adapter_radio_udp::{ResiliencePolicy, ResilientRadioTransport, UdpDownlinkTransport};
use maverick_core::health::{ComponentHealth, HealthState, HealthStatus};
use maverick_core::ports::{DownlinkFrame, RadioTransport};
use maverick_core::{InstallProfile, StoragePressureLevel, StoragePressureSource};
use maverick_domain::{identifiers::Eui64, DevAddr, GatewayEui};

use crate::cli_constants::{
    HEALTH_COMPONENT_STORAGE, RADIO_PROBE_PAYLOAD_BYTE, RECENT_ERRORS_NOT_WIRED_MESSAGE,
    STORAGE_OPEN_FAILED_PREFIX,
};
use crate::edge_json::{self, RadioProbeOutcome, RecentErrorsStubResponse};
use crate::probe::{health_from_probe, total_disk_bytes_hint, HardwareCapabilities};

pub(crate) fn db_path(data_dir: &Path, edge_db_filename: &str) -> PathBuf {
    data_dir.join(edge_db_filename)
}

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

pub(crate) async fn run_status(data_dir: PathBuf, db_file: &str) {
    let cap = HardwareCapabilities::probe();
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
    let doc = edge_json::status_document(
        &data_dir,
        format!("{:?}", cap.suggested_install_profile()),
        cap.total_memory_bytes,
        storage,
    );
    println!("{}", serde_json::to_string(&doc).expect("status json"));
}

pub(crate) async fn run_health(data_dir: PathBuf, db_file: &str) {
    let cap = HardwareCapabilities::probe();
    let mut components = health_from_probe(&cap).components;
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

pub(crate) fn run_probe() {
    let cap = HardwareCapabilities::probe();
    println!(
        "{}",
        serde_json::to_string_pretty(&cap).expect("probe json")
    );
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
