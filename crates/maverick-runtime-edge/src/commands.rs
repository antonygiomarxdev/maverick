//! CLI command handlers (keeps `main` thin).

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use maverick_adapter_persistence_sqlite::{SqlitePersistence, SqlitePersistenceOptions};
use maverick_adapter_radio_udp::{ResiliencePolicy, ResilientRadioTransport, UdpDownlinkTransport};
use maverick_core::health::{ComponentHealth, HealthState, HealthStatus};
use maverick_core::ports::{
    AuditSink, DownlinkFrame, RadioTransport, SessionRepository, UplinkRepository,
};
use maverick_core::protocol::LoRaWAN10xClassA;
use maverick_core::use_cases::IngestUplink;
use maverick_core::{InstallProfile, StoragePressureLevel, StoragePressureSource};
use maverick_domain::{identifiers::Eui64, DevAddr, GatewayEui};
use tokio::net::UdpSocket;

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

pub(crate) async fn run_radio_ingest_once(
    bind: String,
    timeout_ms: u64,
    data_dir: PathBuf,
    db_file: &str,
) {
    let socket = match UdpSocket::bind(bind.as_str()).await {
        Ok(v) => v,
        Err(e) => {
            let out = edge_json::radio_ingest_result(
                bind.as_str(),
                timeout_ms,
                0,
                0,
                0,
                1,
                Some(format!("bind failed: {e}")),
            );
            println!("{}", serde_json::to_string(&out).expect("ingest result"));
            return;
        }
    };
    let mut buf = vec![0_u8; 4096];
    let timeout = Duration::from_millis(timeout_ms.max(1));
    let recv_res = tokio::time::timeout(timeout, socket.recv_from(&mut buf)).await;
    let (n, _addr) = match recv_res {
        Ok(Ok(v)) => v,
        Ok(Err(e)) => {
            let out = edge_json::radio_ingest_result(
                bind.as_str(),
                timeout_ms,
                0,
                0,
                0,
                1,
                Some(format!("recv failed: {e}")),
            );
            println!("{}", serde_json::to_string(&out).expect("ingest result"));
            return;
        }
        Err(_) => {
            let out = edge_json::radio_ingest_result(
                bind.as_str(),
                timeout_ms,
                0,
                0,
                0,
                0,
                Some("listen timeout".to_string()),
            );
            println!("{}", serde_json::to_string(&out).expect("ingest result"));
            return;
        }
    };

    let dbp = db_path(&data_dir, db_file);
    let cap = HardwareCapabilities::probe();
    let profile = cap.suggested_install_profile();
    let policy = profile.default_storage_policy();
    let store = match SqlitePersistence::open(&dbp, policy, sqlite_opts()) {
        Ok(v) => Arc::new(v),
        Err(e) => {
            let out = edge_json::radio_ingest_result(
                bind.as_str(),
                timeout_ms,
                1,
                0,
                0,
                1,
                Some(format!("storage open failed: {e}")),
            );
            println!("{}", serde_json::to_string(&out).expect("ingest result"));
            return;
        }
    };
    let sessions: Arc<dyn SessionRepository> = store.clone();
    let uplinks: Arc<dyn UplinkRepository> = store.clone();
    let audit: Arc<dyn AuditSink> = store.clone();
    let svc = IngestUplink {
        sessions,
        uplinks,
        audit,
        protocol: Arc::new(LoRaWAN10xClassA),
    };

    let parsed = match maverick_adapter_radio_udp::parse_push_data(&buf[..n]) {
        Ok(v) => v,
        Err(e) => {
            let out = edge_json::radio_ingest_result(
                bind.as_str(),
                timeout_ms,
                1,
                0,
                0,
                1,
                Some(e.to_string()),
            );
            println!("{}", serde_json::to_string(&out).expect("ingest result"));
            return;
        }
    };

    let mut ingested = 0_usize;
    let mut failed = 0_usize;
    for obs in parsed.observations {
        match svc.execute(obs).await {
            Ok(()) => ingested += 1,
            Err(_) => failed += 1,
        }
    }

    let out = edge_json::radio_ingest_result(
        bind.as_str(),
        timeout_ms,
        1,
        ingested + failed,
        ingested,
        failed,
        None,
    );
    println!("{}", serde_json::to_string(&out).expect("ingest result"));
}
