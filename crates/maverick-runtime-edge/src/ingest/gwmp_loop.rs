//! Radio uplink receive loop: GWMP/UDP or SPI concentrator (`[radio]` in `lns-config.toml`).

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use maverick_adapter_persistence_sqlite::{SqlitePersistence, SqlitePersistenceOptions};
use maverick_adapter_radio_udp::GwmpUdpIngressBackend;
use maverick_core::ports::{
    AuditSink, SessionRepository, UplinkIngressBackend, UplinkReceive, UplinkRepository,
};
use maverick_core::protocol::LoRaWAN10xClassA;
use maverick_core::use_cases::IngestUplink;

use crate::cli_constants::DEFAULT_LNS_CONFIG_PATH;
use crate::edge_json::{self, RadioIngestCounters};
use crate::ingest::lns_guard::ingest_uplink_with_lns_guard;
use crate::paths::db_path;
use crate::probe::{total_disk_bytes_hint, HardwareCapabilities};
use crate::radio_ingest_selection::{
    build_uplink_source, resolve_radio_ingest, RadioIngestSelection,
};
use crate::runtime_capabilities::{self, RuntimeCapabilityReport};
use crate::watchdog::{send_ready, send_stopping, send_watchdog_ping};

#[cfg(feature = "spi")]
use maverick_adapter_radio_spi::SpiConcentratorIngressBackend;

fn sqlite_opts() -> SqlitePersistenceOptions {
    SqlitePersistenceOptions {
        total_disk_bytes: total_disk_bytes_hint(),
        ..SqlitePersistenceOptions::default()
    }
}

fn listen_label(selection: &RadioIngestSelection) -> &str {
    match selection {
        RadioIngestSelection::Udp { bind } => bind.as_str(),
        RadioIngestSelection::Spi { spi_path } => spi_path.as_str(),
        RadioIngestSelection::AutoSpi { spi_path, .. } => spi_path.as_str(),
        RadioIngestSelection::AutoUdp { bind, .. } => bind.as_str(),
    }
}

fn trace_ingest_identity(selection: &RadioIngestSelection) {
    match selection {
        RadioIngestSelection::Udp { .. } => {
            let backend = GwmpUdpIngressBackend;
            tracing::info!(
                backend_id = backend.id(),
                backend_kind = ?backend.kind(),
                "uplink ingress backend (GWMP/UDP)"
            );
        }
        #[cfg(feature = "spi")]
        RadioIngestSelection::Spi { .. } => {
            let backend = SpiConcentratorIngressBackend;
            tracing::info!(
                backend_id = backend.id(),
                backend_kind = ?backend.kind(),
                "uplink ingress backend (SPI concentrator)"
            );
        }
        #[cfg(not(feature = "spi"))]
        RadioIngestSelection::Spi { .. } => {}
        RadioIngestSelection::AutoSpi { .. } => {
            #[cfg(feature = "spi")]
            {
                let backend = SpiConcentratorIngressBackend;
                tracing::info!(
                    backend_id = backend.id(),
                    backend_kind = ?backend.kind(),
                    "uplink ingress backend (SPI concentrator, auto-detected)"
                );
            }
        }
        RadioIngestSelection::AutoUdp { .. } => {
            let backend = GwmpUdpIngressBackend;
            tracing::info!(
                backend_id = backend.id(),
                backend_kind = ?backend.kind(),
                "uplink ingress backend (GWMP/UDP, SPI auto-detect fallback)"
            );
        }
    }
}

pub(crate) async fn run_radio_ingest_once(
    bind: String,
    timeout_ms: u64,
    data_dir: PathBuf,
    db_file: &str,
) {
    let timeout = Duration::from_millis(timeout_ms.max(1));
    let lns_path = Path::new(DEFAULT_LNS_CONFIG_PATH);
    let selection = match resolve_radio_ingest(lns_path, bind.clone()) {
        Ok(s) => s,
        Err(e) => {
            let out =
                edge_json::radio_ingest_result(bind.as_str(), timeout_ms, 0, 0, 0, 1, Some(e));
            println!("{}", serde_json::to_string(&out).expect("ingest result"));
            return;
        }
    };
    let label = listen_label(&selection).to_string();
    let source = match build_uplink_source(selection.clone(), timeout).await {
        Ok(s) => s,
        Err(e) => {
            let out = edge_json::radio_ingest_result(
                label.as_str(),
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

    let dbp = db_path(&data_dir, db_file);
    let cap = HardwareCapabilities::probe();
    let profile = cap.suggested_install_profile();
    let policy = profile.default_storage_policy();
    let store = match SqlitePersistence::open(&dbp, policy, sqlite_opts()) {
        Ok(v) => Arc::new(v),
        Err(e) => {
            let out = edge_json::radio_ingest_result(
                label.as_str(),
                timeout_ms,
                0,
                0,
                0,
                1,
                Some(format!("storage open failed: {e}")),
            );
            println!("{}", serde_json::to_string(&out).expect("ingest result"));
            return;
        }
    };

    trace_ingest_identity(&selection);
    let report = RuntimeCapabilityReport::build(bind.clone(), Some(lns_path));
    runtime_capabilities::log_ingest_capability_report(&report);

    let sessions: Arc<dyn SessionRepository> = store.clone();
    let uplinks: Arc<dyn UplinkRepository> = store.clone();
    let audit: Arc<dyn AuditSink> = store.clone();
    let svc = IngestUplink {
        sessions,
        uplinks,
        audit,
        protocol: Arc::new(LoRaWAN10xClassA),
    };

    let recv = match source.next_batch().await {
        Ok(r) => r,
        Err(e) => {
            let out = edge_json::radio_ingest_result(
                label.as_str(),
                timeout_ms,
                0,
                0,
                0,
                1,
                Some(format!("{e}")),
            );
            println!("{}", serde_json::to_string(&out).expect("ingest result"));
            return;
        }
    };

    match recv {
        UplinkReceive::Idle => {
            let out = edge_json::radio_ingest_result(
                label.as_str(),
                timeout_ms,
                0,
                0,
                0,
                0,
                Some("listen timeout".to_string()),
            );
            println!("{}", serde_json::to_string(&out).expect("ingest result"));
        }
        UplinkReceive::Observations(observations) => {
            let mut ingested = 0_usize;
            let mut failed = 0_usize;
            for obs in observations {
                match ingest_uplink_with_lns_guard(&store, &svc, obs).await {
                    Ok(()) => ingested += 1,
                    Err(e) => {
                        failed += 1;
                        tracing::warn!(error = %e, "ingest observation failed");
                    }
                }
            }
            let out = edge_json::radio_ingest_result(
                label.as_str(),
                timeout_ms,
                1,
                ingested + failed,
                ingested,
                failed,
                None,
            );
            println!("{}", serde_json::to_string(&out).expect("ingest result"));
        }
    }
}

pub(crate) async fn run_radio_ingest_supervised(
    bind: String,
    read_timeout_ms: u64,
    max_messages: u32,
    data_dir: PathBuf,
    db_file: &str,
) {
    let timeout = Duration::from_millis(read_timeout_ms.max(1));
    let lns_path = Path::new(DEFAULT_LNS_CONFIG_PATH);
    let selection = match resolve_radio_ingest(lns_path, bind.clone()) {
        Ok(s) => s,
        Err(e) => {
            let out = edge_json::radio_ingest_loop_result(
                bind.as_str(),
                read_timeout_ms,
                RadioIngestCounters {
                    looped: true,
                    failed: 1,
                    ..RadioIngestCounters::default()
                },
                Some(e),
            );
            println!(
                "{}",
                serde_json::to_string(&out).expect("ingest-loop result")
            );
            std::process::exit(1);
        }
    };
    let label = listen_label(&selection).to_string();
    let source = match build_uplink_source(selection.clone(), timeout).await {
        Ok(s) => s,
        Err(e) => {
            let out = edge_json::radio_ingest_loop_result(
                label.as_str(),
                read_timeout_ms,
                RadioIngestCounters {
                    looped: true,
                    failed: 1,
                    ..RadioIngestCounters::default()
                },
                Some(format!("bind failed: {e}")),
            );
            println!(
                "{}",
                serde_json::to_string(&out).expect("ingest-loop result")
            );
            std::process::exit(1);
        }
    };
    let dbp = db_path(&data_dir, db_file);
    let cap = HardwareCapabilities::probe();
    let profile = cap.suggested_install_profile();
    let policy = profile.default_storage_policy();
    let store = match SqlitePersistence::open(&dbp, policy, sqlite_opts()) {
        Ok(v) => Arc::new(v),
        Err(e) => {
            let out = edge_json::radio_ingest_loop_result(
                label.as_str(),
                read_timeout_ms,
                RadioIngestCounters {
                    looped: true,
                    failed: 1,
                    ..RadioIngestCounters::default()
                },
                Some(format!("storage open failed: {e}")),
            );
            println!(
                "{}",
                serde_json::to_string(&out).expect("ingest-loop result")
            );
            std::process::exit(1);
        }
    };

    trace_ingest_identity(&selection);
    let report = RuntimeCapabilityReport::build(bind.clone(), Some(lns_path));
    runtime_capabilities::log_ingest_capability_report(&report);

    if let Err(e) = send_ready() {
        tracing::warn!(error = %e, "failed to send READY=1 to systemd");
    }

    let watchdog_handle = tokio::spawn(async {
        loop {
            tokio::time::sleep(Duration::from_secs(15)).await;
            if let Err(e) = send_watchdog_ping() {
                tracing::warn!(error = %e, "watchdog ping failed");
            }
        }
    });

    let sessions: Arc<dyn SessionRepository> = store.clone();
    let uplinks: Arc<dyn UplinkRepository> = store.clone();
    let audit: Arc<dyn AuditSink> = store.clone();
    let svc = IngestUplink {
        sessions,
        uplinks,
        audit,
        protocol: Arc::new(LoRaWAN10xClassA),
    };

    let mut received = 0_usize;
    let mut parsed = 0_usize;
    let mut ingested = 0_usize;
    let mut failed = 0_usize;
    let mut completed_iterations = 0_u32;
    while max_messages == 0 || completed_iterations < max_messages {
        let batch = match source.next_batch().await {
            Ok(b) => b,
            Err(_) => {
                failed += 1;
                completed_iterations += 1;
                continue;
            }
        };
        match batch {
            UplinkReceive::Idle => {
                completed_iterations += 1;
            }
            UplinkReceive::Observations(observations) => {
                received += 1;
                parsed += observations.len();
                for obs in observations {
                    match ingest_uplink_with_lns_guard(&store, &svc, obs).await {
                        Ok(()) => ingested += 1,
                        Err(e) => {
                            failed += 1;
                            tracing::warn!(error = %e, "ingest observation failed");
                        }
                    }
                }
                completed_iterations += 1;
            }
        }
    }
    watchdog_handle.abort();
    let _ = send_stopping();
    let out = edge_json::radio_ingest_loop_result(
        label.as_str(),
        read_timeout_ms,
        RadioIngestCounters {
            looped: true,
            received,
            parsed,
            ingested,
            failed,
        },
        None,
    );
    println!(
        "{}",
        serde_json::to_string(&out).expect("ingest-loop result")
    );
}
