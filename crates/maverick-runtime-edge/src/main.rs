//! Edge runtime entrypoint: local CLI for visibility baseline (v1).

use clap::{Parser, Subcommand};
use maverick_core::InstallProfile;
use tracing_subscriber::EnvFilter;

mod probe;

use probe::{health_from_probe, HardwareCapabilities};

#[derive(Parser)]
#[command(name = "maverick-edge")]
#[command(about = "Maverick offline-first edge runtime")]
struct Cli {
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
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    match cli.command {
        Commands::Status => {
            let cap = HardwareCapabilities::probe();
            println!(
                "{}",
                serde_json::json!({
                    "role": "edge",
                    "suggested_profile": format!("{:?}", cap.suggested_install_profile()),
                    "memory_bytes": cap.total_memory_bytes,
                })
            );
        }
        Commands::Health => {
            let cap = HardwareCapabilities::probe();
            let h = health_from_probe(&cap);
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
    }
}
