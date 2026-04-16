//! GWMP/UDP uplink receive loop (composition root selects this backend today).

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use maverick_adapter_persistence_sqlite::{SqlitePersistence, SqlitePersistenceOptions};
use maverick_adapter_radio_udp::{GwmpUdpIngressBackend, GwmpUdpUplinkSource};
use maverick_core::ports::{
    AuditSink, SessionRepository, UplinkIngressBackend, UplinkReceive, UplinkRepository, UplinkSource,
};
use maverick_core::protocol::LoRaWAN10xClassA;
use maverick_core::use_cases::IngestUplink;

use crate::cli_constants::DEFAULT_LNS_CONFIG_PATH;
use crate::edge_json::{self, RadioIngestCounters};
use crate::ingest::lns_guard::ingest_uplink_with_lns_guard;
use crate::paths::db_path;
use crate::probe::{total_disk_bytes_hint, HardwareCapabilities};
use crate::runtime_capabilities;

fn sqlite_opts() -> SqlitePersistenceOptions {
    SqlitePersistenceOptions {
        total_disk_bytes: total_disk_bytes_hint(),
        ..SqlitePersistenceOptions::default()
    }
}

pub(crate) async fn run_radio_ingest_once(
    bind: String,
    timeout_ms: u64,
    data_dir: PathBuf,
    db_file: &str,
) {
    let timeout = Duration::from_millis(timeout_ms.max(1));
    let source = match GwmpUdpUplinkSource::bind(bind.as_str(), timeout).await {
        Ok(s) => s,
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

    let backend = GwmpUdpIngressBackend;
    tracing::info!(
        backend_id = backend.id(),
        backend_kind = ?backend.kind(),
        "uplink ingress backend (GWMP/UDP)"
    );
    runtime_capabilities::log_startup_snapshot(
        &bind,
        Some(std::path::Path::new(DEFAULT_LNS_CONFIG_PATH)),
    );

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
                bind.as_str(),
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
                bind.as_str(),
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
    let source = match GwmpUdpUplinkSource::bind(bind.as_str(), timeout).await {
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
                bind.as_str(),
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

    let backend = GwmpUdpIngressBackend;
    tracing::info!(
        backend_id = backend.id(),
        backend_kind = ?backend.kind(),
        "uplink ingress backend (GWMP/UDP)"
    );
    runtime_capabilities::log_startup_snapshot(
        &bind,
        Some(std::path::Path::new(DEFAULT_LNS_CONFIG_PATH)),
    );

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
    let out = edge_json::radio_ingest_loop_result(
        bind.as_str(),
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
