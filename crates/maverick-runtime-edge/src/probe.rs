//! Best-effort hardware / OS capability probe for install-time and runtime profile hints.

use maverick_core::health::{ComponentHealth, HealthState, HealthStatus};
use serde::Serialize;
use sysinfo::{Disks, System};

const KIB: u64 = 1024;
const MIB: u64 = 1024 * KIB;
const GIB: u64 = 1024 * MIB;
const MEMORY_BYTES_512_MIB: u64 = 512 * MIB;
const MEMORY_BYTES_2_GIB: u64 = 2 * GIB;

const HEALTH_COMPONENT_MEMORY_PROBE: &str = "memory_probe";
const HEALTH_DETAIL_TOTAL_BYTES_KEY: &str = "total_bytes";

#[derive(Debug, Clone, Serialize)]
pub struct HardwareCapabilities {
    pub total_memory_bytes: u64,
    pub os_name: Option<String>,
    pub os_version: Option<String>,
}

impl HardwareCapabilities {
    pub fn probe() -> Self {
        let mut sys = System::new_all();
        sys.refresh_memory();
        Self {
            total_memory_bytes: sys.total_memory(),
            os_name: System::name(),
            os_version: System::os_version(),
        }
    }

    /// Map coarse memory buckets to suggested install profile (operator may override).
    pub fn suggested_install_profile(&self) -> maverick_core::InstallProfile {
        if self.total_memory_bytes < MEMORY_BYTES_512_MIB {
            maverick_core::InstallProfile::Constrained
        } else if self.total_memory_bytes < MEMORY_BYTES_2_GIB {
            maverick_core::InstallProfile::Balanced
        } else {
            maverick_core::InstallProfile::HighCapacity
        }
    }
}

pub fn health_from_probe(cap: &HardwareCapabilities) -> HealthState {
    let mem_status = if cap.total_memory_bytes == 0 {
        HealthStatus::Degraded
    } else {
        HealthStatus::Healthy
    };
    HealthState::new(vec![ComponentHealth {
        name: HEALTH_COMPONENT_MEMORY_PROBE.to_string(),
        status: mem_status,
        detail: Some(format!(
            "{HEALTH_DETAIL_TOTAL_BYTES_KEY}={}",
            cap.total_memory_bytes
        )),
    }])
}

/// Best-effort total disk capacity for storage pressure ratios (first refreshed disk with non-zero total).
pub fn total_disk_bytes_hint() -> Option<u64> {
    let disks = Disks::new_with_refreshed_list();
    disks.iter().map(|d| d.total_space()).find(|t| *t > 0)
}
