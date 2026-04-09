use std::env;

use crate::error::{AppError, Result};
use crate::storage_profile::{
    detect_hardware_snapshot, resolve_profile_with_hardware_guard, StorageLimits, StorageProfile,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeConfig {
    pub http_bind_addr: String,
    pub udp_bind_addr: String,
    pub database_path: String,
    pub log_filter: String,
    pub event_bus_capacity: usize,
    pub udp_max_datagram_size: usize,
    pub storage_profile: StorageProfile,
    pub storage_limits: StorageLimits,
}

impl RuntimeConfig {
    pub fn from_env() -> Result<Self> {
        let mut config = Self::default();
        let mut limits_overridden = false;

        if let Ok(value) = env::var("MAVERICK_HTTP_BIND_ADDR") {
            config.http_bind_addr = value;
        }

        if let Ok(value) = env::var("MAVERICK_UDP_BIND_ADDR") {
            config.udp_bind_addr = value;
        }

        if let Ok(value) = env::var("MAVERICK_DB_PATH") {
            config.database_path = value;
        }

        if let Ok(value) = env::var("MAVERICK_LOG_FILTER") {
            config.log_filter = value;
        }

        if let Ok(value) = env::var("MAVERICK_EVENT_BUS_CAPACITY") {
            config.event_bus_capacity = value.parse().map_err(|err| {
                AppError::Config(format!(
                    "invalid MAVERICK_EVENT_BUS_CAPACITY '{value}': {err}"
                ))
            })?;
        }

        if let Ok(value) = env::var("MAVERICK_UDP_MAX_DATAGRAM_SIZE") {
            config.udp_max_datagram_size = value.parse().map_err(|err| {
                AppError::Config(format!(
                    "invalid MAVERICK_UDP_MAX_DATAGRAM_SIZE '{value}': {err}"
                ))
            })?;
        }

        if let Ok(value) = env::var("MAVERICK_STORAGE_PROFILE") {
            config.storage_profile = StorageProfile::parse(&value).ok_or_else(|| {
                AppError::Config(format!(
                    "invalid MAVERICK_STORAGE_PROFILE '{value}': expected auto|high|mid|extreme"
                ))
            })?;
        }

        if let Ok(value) = env::var("MAVERICK_STORAGE_MAX_LOCAL_MB") {
            config.storage_limits.max_local_storage_mb = value.parse().map_err(|err| {
                AppError::Config(format!(
                    "invalid MAVERICK_STORAGE_MAX_LOCAL_MB '{value}': {err}"
                ))
            })?;
            limits_overridden = true;
        }

        if let Ok(value) = env::var("MAVERICK_STORAGE_RETENTION_DAYS") {
            config.storage_limits.retention_days = value.parse().map_err(|err| {
                AppError::Config(format!(
                    "invalid MAVERICK_STORAGE_RETENTION_DAYS '{value}': {err}"
                ))
            })?;
            limits_overridden = true;
        }

        if let Ok(value) = env::var("MAVERICK_STORAGE_BATCH_COMMIT_SIZE") {
            config.storage_limits.batch_commit_size = value.parse().map_err(|err| {
                AppError::Config(format!(
                    "invalid MAVERICK_STORAGE_BATCH_COMMIT_SIZE '{value}': {err}"
                ))
            })?;
            limits_overridden = true;
        }

        if let Ok(value) = env::var("MAVERICK_STORAGE_BATCH_COMMIT_INTERVAL_MS") {
            config.storage_limits.batch_commit_interval_ms = value.parse().map_err(|err| {
                AppError::Config(format!(
                    "invalid MAVERICK_STORAGE_BATCH_COMMIT_INTERVAL_MS '{value}': {err}"
                ))
            })?;
            limits_overridden = true;
        }

        if let Ok(value) = env::var("MAVERICK_STORAGE_CIRCULAR_CAPACITY") {
            config.storage_limits.circular_buffer_capacity = value.parse().map_err(|err| {
                AppError::Config(format!(
                    "invalid MAVERICK_STORAGE_CIRCULAR_CAPACITY '{value}': {err}"
                ))
            })?;
            limits_overridden = true;
        }

        if config.database_path.trim().is_empty() {
            return Err(AppError::Config(
                "database path cannot be empty".to_string(),
            ));
        }

        if !limits_overridden {
            config.storage_limits = StorageLimits::for_profile(config.resolve_storage_profile());
        }

        Ok(config)
    }

    pub fn resolve_storage_profile(&self) -> StorageProfile {
        self.resolve_storage_profile_with_snapshot(detect_hardware_snapshot())
    }

    pub(crate) fn resolve_storage_profile_with_snapshot(
        &self,
        snapshot: Option<crate::storage_profile::HardwareSnapshot>,
    ) -> StorageProfile {
        resolve_profile_with_hardware_guard(self.storage_profile, snapshot)
    }
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            http_bind_addr: "0.0.0.0:8080".to_string(),
            udp_bind_addr: "0.0.0.0:1700".to_string(),
            database_path: "maverick.db".to_string(),
            log_filter: "maverick_core=info,tower_http=info".to_string(),
            event_bus_capacity: 256,
            udp_max_datagram_size: 4096,
            storage_profile: StorageProfile::Auto,
            storage_limits: StorageLimits::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RuntimeConfig;
    use crate::storage_profile::{HardwareSnapshot, StorageProfile};

    #[test]
    fn default_runtime_config_matches_foundation_expectations() {
        let config = RuntimeConfig::default();

        assert_eq!(config.http_bind_addr, "0.0.0.0:8080");
        assert_eq!(config.udp_bind_addr, "0.0.0.0:1700");
        assert_eq!(config.database_path, "maverick.db");
        assert_eq!(config.event_bus_capacity, 256);
        assert_eq!(config.udp_max_datagram_size, 4096);
        assert_eq!(config.storage_profile, StorageProfile::Auto);
        assert!(config.storage_limits.max_local_storage_mb > 0);
    }

    #[test]
    fn auto_storage_profile_resolves_to_extreme_for_low_memory_snapshot() {
        let config = RuntimeConfig::default();
        let resolved = config.resolve_storage_profile_with_snapshot(Some(HardwareSnapshot {
            total_memory_mb: 64,
        }));

        assert_eq!(resolved, StorageProfile::Extreme);
    }

    #[test]
    fn explicit_env_profile_is_used_if_hardware_supports_it() {
        let config = RuntimeConfig {
            storage_profile: StorageProfile::Mid,
            ..Default::default()
        };

        let resolved = config.resolve_storage_profile_with_snapshot(Some(HardwareSnapshot {
            total_memory_mb: 1024,
        }));

        assert_eq!(resolved, StorageProfile::Mid);
    }

    #[test]
    fn explicit_env_profile_is_downgraded_if_hardware_cannot_support_it() {
        let config = RuntimeConfig {
            storage_profile: StorageProfile::High,
            ..Default::default()
        };

        let resolved = config.resolve_storage_profile_with_snapshot(Some(HardwareSnapshot {
            total_memory_mb: 64,
        }));

        assert_eq!(resolved, StorageProfile::Extreme);
    }
}
