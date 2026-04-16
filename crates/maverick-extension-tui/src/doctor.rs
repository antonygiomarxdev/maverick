//! Operator dashboard: probe + status/health JSON + recommendations.

use serde::Deserialize;

use crate::config::TuiConfig;
use crate::console_ui::UiStyle;
use crate::edge_runner::run_edge_json_command;
use crate::profiles::suggested_profile_from_memory;

/// Mirrors `maverick-edge probe` root JSON (`RuntimeCapabilityReport.hardware`).
#[derive(Deserialize)]
struct RuntimeCapabilityProbe {
    hardware: HardwareProbeSummary,
}

#[derive(Deserialize)]
pub(crate) struct HardwareProbeSummary {
    pub(crate) total_memory_bytes: u64,
    pub(crate) os_name: Option<String>,
    pub(crate) os_version: Option<String>,
}

pub(crate) fn probe_edge_capabilities() -> Option<HardwareProbeSummary> {
    let output = crate::edge_runner::maverick_edge_probe_output()?;
    if !output.status.success() {
        return None;
    }
    serde_json::from_slice::<RuntimeCapabilityProbe>(&output.stdout)
        .ok()
        .map(|p| p.hardware)
}

pub(crate) fn run_doctor_dashboard(cfg: &TuiConfig) -> Result<(), String> {
    let style = UiStyle::detect();
    println!();
    println!("{}", style.heading("== Maverick Operator Dashboard =="));

    let probe = probe_edge_capabilities();
    if let Some(p) = probe {
        println!(
            "Host: {} {}",
            p.os_name.clone().unwrap_or_else(|| "unknown".to_string()),
            p.os_version
                .clone()
                .unwrap_or_else(|| "unknown".to_string())
        );
        println!(
            "Memory: {} bytes (suggested profile: {})",
            p.total_memory_bytes,
            suggested_profile_from_memory(p.total_memory_bytes)
        );
    } else {
        println!("Host probe: unavailable");
    }

    let status = run_edge_json_command(cfg, &["status"])?;
    let health = run_edge_json_command(cfg, &["health"])?;

    let suggested = status
        .get("suggested_profile")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let data_dir = status
        .get("data_dir")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let storage_present = status
        .get("storage")
        .and_then(|v| v.get("present"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let overall = health
        .get("overall")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    println!("Runtime data dir: {data_dir}");
    println!("Runtime suggested profile: {suggested}");
    println!("Storage present: {storage_present}");
    println!("Health overall: {overall}");

    println!();
    println!("Recommendations:");
    if overall != "healthy" {
        println!(
            "- Health is not healthy: run 'maverick-edge health' and inspect degraded components."
        );
    } else {
        println!("- Health looks good.");
    }

    if !storage_present {
        println!("- Storage is not initialized yet: start ingest-loop or run runtime to create local db.");
    } else {
        println!("- Storage is present.");
    }

    if suggested.eq_ignore_ascii_case("HighCapacity") && cfg.loop_read_timeout_ms > 1_000 {
        println!("- Host can handle higher throughput; consider 'apply-profile --profile high-capacity'.");
    }

    Ok(())
}
