//! Edge runtime entrypoint: local CLI for visibility baseline (v1).

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use maverick_core::InstallProfile;

pub mod cli_constants;
pub mod commands;
pub mod edge_json;
pub mod ingest;
pub mod paths;
pub mod probe;
pub mod radio_ingest_selection;
pub mod runtime_capabilities;
pub mod watchdog;

use cli_constants::{
    DEFAULT_DATA_DIR, DEFAULT_GWMP_BIND_ADDR, DEFAULT_GWMP_INGEST_TIMEOUT_MS,
    DEFAULT_GWMP_LOOP_MAX_MESSAGES, DEFAULT_GWMP_LOOP_READ_TIMEOUT_MS, DEFAULT_LNS_CONFIG_PATH,
    DEFAULT_RADIO_PROBE_HOST, DEFAULT_RADIO_PROBE_PORT, EDGE_DB_FILENAME,
};
use commands::{
    config, run_health, run_probe, run_radio_downlink_probe, run_recent_errors, run_setup,
    run_status, run_storage_policy, run_storage_pressure,
};
use ingest::{run_radio_ingest_once, run_radio_ingest_supervised};

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
    /// Start setup wizard (interactive) or apply defaults in non-interactive mode
    Setup {
        #[arg(long)]
        non_interactive: bool,
    },
    /// Aggregate health from local probes
    Health,
    /// Print last N stderr-equivalent lines placeholder (structured logs live on disk in full impl)
    RecentErrors {
        #[arg(default_value = "20")]
        lines: usize,
    },
    /// Dump runtime capability report JSON (hardware, radio hints, ingest mode)
    Probe {
        /// Plain-text summary for operators instead of JSON
        #[arg(long)]
        summary: bool,
    },
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
    /// Declarative LNS configuration file ↔ SQLite (`/etc/maverick/lns-config.toml`)
    Config {
        #[command(subcommand)]
        cmd: ConfigCmd,
    },
}

#[derive(Subcommand)]
enum ConfigCmd {
    /// Write a starter `lns-config.toml` (fails if the file exists unless `--force`)
    Init {
        #[arg(long)]
        force: bool,
        #[arg(long, default_value = DEFAULT_LNS_CONFIG_PATH)]
        config_path: PathBuf,
    },
    /// Parse and validate `lns-config.toml` without touching the database
    Validate {
        #[arg(long, default_value = DEFAULT_LNS_CONFIG_PATH)]
        config_path: PathBuf,
    },
    /// Validate file and upsert applications/devices/sessions into SQLite
    Load {
        #[arg(long, default_value = DEFAULT_LNS_CONFIG_PATH)]
        config_path: PathBuf,
    },
    /// Show autoprovision policy plus mirrored applications/devices/pending from SQLite
    Show,
    /// List applications (JSON array)
    ListApps,
    /// List devices (JSON array)
    ListDevices,
    /// List pending DevAddr rows (JSON array)
    ListPending,
    /// Approve a pending device by promoting it into `lns_devices` + `sessions`
    ApproveDevice {
        #[arg(long)]
        dev_eui: String,
        #[arg(long)]
        dev_addr: String,
        #[arg(long)]
        application_id: String,
        #[arg(long, default_value = "EU868")]
        region: String,
    },
    /// Remove a pending DevAddr (does not delete an active device row)
    RejectDevice {
        #[arg(long)]
        dev_addr: String,
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
        #[arg(
            long,
            default_value_t = DEFAULT_GWMP_INGEST_TIMEOUT_MS,
            env = "MAVERICK_GWMP_INGEST_TIMEOUT_MS"
        )]
        timeout_ms: u64,
    },
    /// Supervised ingest loop for local gateway operation (continues on recoverable failures).
    IngestLoop {
        #[arg(long, default_value = DEFAULT_GWMP_BIND_ADDR, env = "MAVERICK_GWMP_BIND")]
        bind: String,
        #[arg(
            long,
            default_value_t = DEFAULT_GWMP_LOOP_READ_TIMEOUT_MS,
            env = "MAVERICK_GWMP_LOOP_READ_TIMEOUT_MS"
        )]
        read_timeout_ms: u64,
        #[arg(
            long,
            default_value_t = DEFAULT_GWMP_LOOP_MAX_MESSAGES,
            env = "MAVERICK_GWMP_LOOP_MAX_MESSAGES",
            help = "Stop after this many UDP receive attempts (0 = run until process exit; use under systemd)"
        )]
        max_messages: u32,
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
        Commands::Setup { non_interactive } => run_setup(non_interactive),
        Commands::Health => run_health(cli.data_dir, db_file).await,
        Commands::RecentErrors { lines } => run_recent_errors(lines),
        Commands::Probe { summary } => run_probe(summary),
        Commands::StoragePolicy { profile } => run_storage_policy(profile.into()),
        Commands::StoragePressure => run_storage_pressure(cli.data_dir, db_file).await,
        Commands::Radio { cmd } => match cmd {
            RadioCmd::DownlinkProbe { host, port } => run_radio_downlink_probe(host, port).await,
            RadioCmd::IngestOnce { bind, timeout_ms } => {
                run_radio_ingest_once(bind, timeout_ms, cli.data_dir, db_file).await
            }
            RadioCmd::IngestLoop {
                bind,
                read_timeout_ms,
                max_messages,
            } => {
                run_radio_ingest_supervised(
                    bind,
                    read_timeout_ms,
                    max_messages,
                    cli.data_dir,
                    db_file,
                )
                .await
            }
        },
        Commands::Config { cmd } => match cmd {
            ConfigCmd::Init { force, config_path } => config::run_config_init(config_path, force),
            ConfigCmd::Validate { config_path } => config::run_config_validate(config_path),
            ConfigCmd::Load { config_path } => {
                config::run_config_load(cli.data_dir, db_file, config_path)
            }
            ConfigCmd::Show => config::run_config_show(cli.data_dir, db_file),
            ConfigCmd::ListApps => config::run_config_list_apps(cli.data_dir, db_file),
            ConfigCmd::ListDevices => config::run_config_list_devices(cli.data_dir, db_file),
            ConfigCmd::ListPending => config::run_config_list_pending(cli.data_dir, db_file),
            ConfigCmd::ApproveDevice {
                dev_eui,
                dev_addr,
                application_id,
                region,
            } => config::run_config_approve_device(
                cli.data_dir,
                db_file,
                dev_eui,
                dev_addr,
                application_id,
                region,
            ),
            ConfigCmd::RejectDevice { dev_addr } => {
                config::run_config_reject_device(cli.data_dir, db_file, dev_addr)
            }
        },
    }
}
