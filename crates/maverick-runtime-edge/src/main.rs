//! Edge runtime entrypoint: local CLI for visibility baseline (v1).

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use maverick_core::InstallProfile;

mod cli_constants;
mod commands;
mod edge_json;
mod probe;

use cli_constants::{
    DEFAULT_DATA_DIR, DEFAULT_GWMP_BIND_ADDR, DEFAULT_GWMP_INGEST_TIMEOUT_MS,
    DEFAULT_RADIO_PROBE_HOST, DEFAULT_RADIO_PROBE_PORT, EDGE_DB_FILENAME,
};
use commands::{
    run_health, run_probe, run_radio_downlink_probe, run_radio_ingest_once, run_recent_errors,
    run_status, run_storage_policy, run_storage_pressure,
};

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
    /// Radio / transport diagnostics (adapter wiring; does not start the full kernel loop)
    Radio {
        #[command(subcommand)]
        cmd: RadioCmd,
    },
}

#[derive(Subcommand)]
enum RadioCmd {
    /// Send a minimal UDP downlink probe through the resilient transport wrapper
    DownlinkProbe {
        #[arg(long, default_value = DEFAULT_RADIO_PROBE_HOST)]
        host: String,
        #[arg(long, default_value_t = DEFAULT_RADIO_PROBE_PORT)]
        port: u16,
    },
    /// Listen for one Semtech PUSH_DATA datagram and ingest observations through core use case.
    IngestOnce {
        #[arg(long, default_value = DEFAULT_GWMP_BIND_ADDR)]
        bind: String,
        #[arg(long, default_value_t = DEFAULT_GWMP_INGEST_TIMEOUT_MS)]
        timeout_ms: u64,
    },
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

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    let db_file = EDGE_DB_FILENAME;
    match cli.command {
        Commands::Status => run_status(cli.data_dir, db_file).await,
        Commands::Health => run_health(cli.data_dir, db_file).await,
        Commands::RecentErrors { lines } => run_recent_errors(lines),
        Commands::Probe => run_probe(),
        Commands::StoragePolicy { profile } => run_storage_policy(profile.into()),
        Commands::StoragePressure => run_storage_pressure(cli.data_dir, db_file).await,
        Commands::Radio { cmd } => match cmd {
            RadioCmd::DownlinkProbe { host, port } => run_radio_downlink_probe(host, port).await,
            RadioCmd::IngestOnce { bind, timeout_ms } => {
                run_radio_ingest_once(bind, timeout_ms, cli.data_dir, db_file).await
            }
        },
    }
}
