//! Best-effort hardware / OS capability probe for install-time and runtime profile hints.

use maverick_core::health::{ComponentHealth, HealthState, HealthStatus};
use serde::Serialize;
use sysinfo::System;

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
        const GB: u64 = 1024 * 1024 * 1024;
        if self.total_memory_bytes < 512 * 1024 * 1024 {
            maverick_core::InstallProfile::Constrained
        } else if self.total_memory_bytes < 2 * GB {
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
        name: "memory_probe".to_string(),
        status: mem_status,
        detail: Some(format!("total_bytes={}", cap.total_memory_bytes)),
    }])
}
