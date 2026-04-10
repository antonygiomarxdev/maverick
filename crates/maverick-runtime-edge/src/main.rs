//! Edge runtime entrypoint: local CLI for visibility baseline (v1).

use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use maverick_adapter_persistence_sqlite::{SqlitePersistence, SqlitePersistenceOptions};
use maverick_core::health::{ComponentHealth, HealthState, HealthStatus};
use maverick_core::{InstallProfile, StoragePressureLevel, StoragePressureSource};
use tracing_subscriber::EnvFilter;

mod cli_constants;
mod probe;

use cli_constants::{DEFAULT_DATA_DIR, EDGE_DB_FILENAME, HEALTH_COMPONENT_STORAGE};
use probe::{health_from_probe, total_disk_bytes_hint, HardwareCapabilities};

#[derive(Parser)]
#[command(name = "maverick-edge")]
#[command(about = "Maverick offline-first edge runtime")]
struct Cli {
    /// Data directory for local SQLite (see `EDGE_DB_FILENAME` in `cli_constants`).
    #[arg(long, global = true, env = "MAVERICK_DATA_DIR", default_value = DEFAULT_DATA_DIR)]
    data_dir: PathBuf,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Show process status summary
    Status,
    /// Aggregate health from local probes
    Health,
    /// Print last N stderr-equivalent lines placeholder (structured logs live on disk in full impl)
    RecentErrors {
        #[arg(default_value = "20")]
        lines: usize,
    },
    /// Dump hardware capability probe JSON
    Probe,
    /// Show effective storage policy for install profile
    StoragePolicy {
        #[arg(value_enum)]
        profile: ProfileArg,
    },
    /// Storage pressure snapshot when `maverick.db` exists under data dir
    StoragePressure,
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum ProfileArg {
    Constrained,
    Balanced,
    HighCapacity,
}

impl From<ProfileArg> for InstallProfile {
    fn from(p: ProfileArg) -> Self {
        match p {
            ProfileArg::Constrained => InstallProfile::Constrained,
            ProfileArg::Balanced => InstallProfile::Balanced,
            ProfileArg::HighCapacity => InstallProfile::HighCapacity,
        }
    }
}

fn storage_level_to_health(level: StoragePressureLevel) -> HealthStatus {
    match level {
        StoragePressureLevel::Normal => HealthStatus::Healthy,
        StoragePressureLevel::Elevated
        | StoragePressureLevel::Critical
        | StoragePressureLevel::HardLimit => HealthStatus::Degraded,
    }
}

fn db_path(data_dir: &Path) -> PathBuf {
    data_dir.join(EDGE_DB_FILENAME)
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    match cli.command {
        Commands::Status => {
            let cap = HardwareCapabilities::probe();
            let dbp = db_path(&cli.data_dir);
            let storage = if dbp.exists() {
                let profile = cap.suggested_install_profile();
                let policy = profile.default_storage_policy();
                let opts = SqlitePersistenceOptions {
                    total_disk_bytes: total_disk_bytes_hint(),
                    ..SqlitePersistenceOptions::default()
                };
                match SqlitePersistence::open(&dbp, policy, opts) {
                    Ok(store) => {
                        let snap = store.pressure_snapshot().await;
                        Some(serde_json::json!({
                            "present": true,
                            "level": snap.level,
                            "db_bytes": snap.db_bytes,
                            "total_disk_bytes": snap.total_disk_bytes,
                            "detail": snap.detail,
                        }))
                    }
                    Err(e) => Some(serde_json::json!({
                        "present": true,
                        "error": e.to_string(),
                    })),
                }
            } else {
                Some(serde_json::json!({ "present": false }))
            };
            println!(
                "{}",
                serde_json::json!({
                    "role": "edge",
                    "data_dir": cli.data_dir,
                    "suggested_profile": format!("{:?}", cap.suggested_install_profile()),
                    "memory_bytes": cap.total_memory_bytes,
                    "storage": storage,
                })
            );
        }
        Commands::Health => {
            let cap = HardwareCapabilities::probe();
            let mut components = health_from_probe(&cap).components;
            let dbp = db_path(&cli.data_dir);
            if dbp.exists() {
                let profile = cap.suggested_install_profile();
                let policy = profile.default_storage_policy();
                let opts = SqlitePersistenceOptions {
                    total_disk_bytes: total_disk_bytes_hint(),
                    ..SqlitePersistenceOptions::default()
                };
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
                            detail: Some(format!("open failed: {e}")),
                        });
                    }
                }
            }
            let h = HealthState::new(components);
            println!("{}", serde_json::to_string_pretty(&h).unwrap());
        }
        Commands::RecentErrors { lines } => {
            println!(
                "{{\"message\":\"recent-errors not yet wired to log file\",\"lines_requested\":{}}}",
                lines
            );
        }
        Commands::Probe => {
            let cap = HardwareCapabilities::probe();
            println!("{}", serde_json::to_string_pretty(&cap).unwrap());
        }
        Commands::StoragePolicy { profile } => {
            let p: InstallProfile = profile.into();
            let pol = p.default_storage_policy();
            println!("{}", serde_json::to_string_pretty(&pol).unwrap());
        }
        Commands::StoragePressure => {
            let dbp = db_path(&cli.data_dir);
            if !dbp.exists() {
                println!(
                    "{}",
                    serde_json::json!({
                        "present": false,
                        "data_dir": cli.data_dir,
                    })
                );
                return;
            }
            let cap = HardwareCapabilities::probe();
            let profile = cap.suggested_install_profile();
            let policy = profile.default_storage_policy();
            let opts = SqlitePersistenceOptions {
                total_disk_bytes: total_disk_bytes_hint(),
                ..SqlitePersistenceOptions::default()
            };
            match SqlitePersistence::open(&dbp, policy, opts) {
                Ok(store) => {
                    let snap = store.pressure_snapshot().await;
                    println!("{}", serde_json::to_string_pretty(&snap).unwrap());
                }
                Err(e) => {
                    println!("{{\"error\":{:?}}}", e.to_string());
                }
            }
        }
    }
}
