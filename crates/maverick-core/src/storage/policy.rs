use serde::{Deserialize, Serialize};

/// Selected at install time; drives retention and circular behavior under pressure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallProfile {
    Constrained,
    Balanced,
    HighCapacity,
}

/// Data criticality tier for hybrid retention (oldest purged first within tier when under pressure).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetentionTier {
    /// Must survive longest under normal pressure; still subject to hard circular if configured.
    Critical,
    Operational,
    Telemetry,
}

/// Operator-visible storage pressure (drives health degradation).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StoragePressureLevel {
    Normal,
    Elevated,
    Critical,
    HardLimit,
}

/// Hybrid storage policy: tiered retention + optional circular rollover at hard limit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoragePolicy {
    /// When true, at hard storage limit the runtime may overwrite oldest records (any tier) to keep operating.
    pub circular_at_hard_limit: bool,
    /// Max fraction of disk (0.0–1.0) before elevated pressure (informational threshold).
    pub elevated_use_ratio: f32,
    /// Max fraction before critical pressure.
    pub critical_use_ratio: f32,
    /// Approximate max retained records per tier (adapter interprets in persistence).
    pub max_records_telemetry: u64,
    pub max_records_operational: u64,
    pub max_records_critical: u64,
}

impl Default for StoragePolicy {
    fn default() -> Self {
        HybridRetentionDefaults::balanced().into_policy(true)
    }
}

/// Named presets for install profiles.
pub struct HybridRetentionDefaults;

impl HybridRetentionDefaults {
    pub fn constrained() -> Self {
        Self
    }

    pub fn balanced() -> Self {
        Self
    }

    pub fn high_capacity() -> Self {
        Self
    }

    pub fn into_policy(self, circular_at_hard_limit: bool) -> StoragePolicy {
        let _ = self;
        StoragePolicy {
            circular_at_hard_limit,
            elevated_use_ratio: 0.75,
            critical_use_ratio: 0.9,
            max_records_telemetry: 50_000,
            max_records_operational: 200_000,
            max_records_critical: 500_000,
        }
    }
}

impl InstallProfile {
    pub fn default_storage_policy(self) -> StoragePolicy {
        match self {
            Self::Constrained => StoragePolicy {
                circular_at_hard_limit: true,
                elevated_use_ratio: 0.65,
                critical_use_ratio: 0.85,
                max_records_telemetry: 5_000,
                max_records_operational: 20_000,
                max_records_critical: 50_000,
            },
            Self::Balanced => HybridRetentionDefaults::balanced().into_policy(true),
            Self::HighCapacity => StoragePolicy {
                circular_at_hard_limit: false,
                elevated_use_ratio: 0.8,
                critical_use_ratio: 0.92,
                max_records_telemetry: 200_000,
                max_records_operational: 800_000,
                max_records_critical: 2_000_000,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_profile_policies_differ_by_circular_default() {
        let p = InstallProfile::HighCapacity.default_storage_policy();
        assert!(!p.circular_at_hard_limit);
        let q = InstallProfile::Constrained.default_storage_policy();
        assert!(q.circular_at_hard_limit);
    }

    #[test]
    fn constrained_has_tighter_telemetry_cap_than_balanced() {
        let c = InstallProfile::Constrained.default_storage_policy();
        let b = InstallProfile::Balanced.default_storage_policy();
        assert!(c.max_records_telemetry < b.max_records_telemetry);
    }
}
