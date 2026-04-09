#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageProfile {
    Auto,
    High,
    Mid,
    Extreme,
}

impl StorageProfile {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "auto" => Some(Self::Auto),
            "high" => Some(Self::High),
            "mid" => Some(Self::Mid),
            "extreme" => Some(Self::Extreme),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::High => "high",
            Self::Mid => "mid",
            Self::Extreme => "extreme",
        }
    }
}

impl std::fmt::Display for StorageProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageLimits {
    pub max_local_storage_mb: u64,
    pub retention_days: u32,
    pub batch_commit_size: usize,
    pub batch_commit_interval_ms: u64,
    pub circular_buffer_capacity: usize,
}

impl StorageLimits {
    pub fn for_profile(profile: StorageProfile) -> Self {
        match profile {
            StorageProfile::High => Self {
                max_local_storage_mb: 4096,
                retention_days: 30,
                batch_commit_size: 512,
                batch_commit_interval_ms: 500,
                circular_buffer_capacity: 100_000,
            },
            StorageProfile::Mid => Self {
                max_local_storage_mb: 512,
                retention_days: 14,
                batch_commit_size: 256,
                batch_commit_interval_ms: 1000,
                circular_buffer_capacity: 25_000,
            },
            StorageProfile::Extreme => Self {
                max_local_storage_mb: 64,
                retention_days: 3,
                batch_commit_size: 64,
                batch_commit_interval_ms: 2000,
                circular_buffer_capacity: 5_000,
            },
            StorageProfile::Auto => Self::for_profile(StorageProfile::Mid),
        }
    }
}

impl Default for StorageLimits {
    fn default() -> Self {
        Self::for_profile(StorageProfile::Mid)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HardwareSnapshot {
    pub total_memory_mb: u64,
}

pub fn detect_hardware_snapshot() -> Option<HardwareSnapshot> {
    let mut system = sysinfo::System::new();
    system.refresh_memory();
    let total_memory_kib = system.total_memory();

    if total_memory_kib == 0 {
        return None;
    }

    Some(HardwareSnapshot {
        total_memory_mb: total_memory_kib / 1024,
    })
}

pub fn resolve_auto_profile(snapshot: Option<HardwareSnapshot>) -> StorageProfile {
    let Some(snapshot) = snapshot else {
        return StorageProfile::Mid;
    };

    if snapshot.total_memory_mb < 128 {
        StorageProfile::Extreme
    } else if snapshot.total_memory_mb < 512 {
        StorageProfile::Mid
    } else {
        StorageProfile::High
    }
}

pub fn resolve_profile_with_hardware_guard(
    requested: StorageProfile,
    snapshot: Option<HardwareSnapshot>,
) -> StorageProfile {
    if requested == StorageProfile::Auto {
        return resolve_auto_profile(snapshot);
    }

    let Some(snapshot) = snapshot else {
        // If hardware cannot be detected, honor explicit env selection.
        return requested;
    };

    let max_supported = resolve_auto_profile(Some(snapshot));
    if profile_rank(requested) <= profile_rank(max_supported) {
        requested
    } else {
        max_supported
    }
}

fn profile_rank(profile: StorageProfile) -> u8 {
    match profile {
        StorageProfile::Extreme => 0,
        StorageProfile::Mid => 1,
        StorageProfile::High => 2,
        StorageProfile::Auto => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_all_variants() {
        assert_eq!(StorageProfile::parse("auto"), Some(StorageProfile::Auto));
        assert_eq!(StorageProfile::parse("high"), Some(StorageProfile::High));
        assert_eq!(StorageProfile::parse("mid"), Some(StorageProfile::Mid));
        assert_eq!(
            StorageProfile::parse("extreme"),
            Some(StorageProfile::Extreme)
        );
        assert_eq!(StorageProfile::parse("HIGH"), Some(StorageProfile::High));
        assert_eq!(StorageProfile::parse("unknown"), None);
    }

    #[test]
    fn resolve_none_returns_mid() {
        assert_eq!(resolve_auto_profile(None), StorageProfile::Mid);
    }

    #[test]
    fn resolve_64mb_returns_extreme() {
        let snap = HardwareSnapshot {
            total_memory_mb: 64,
        };
        assert_eq!(resolve_auto_profile(Some(snap)), StorageProfile::Extreme);
    }

    #[test]
    fn resolve_256mb_returns_mid() {
        let snap = HardwareSnapshot {
            total_memory_mb: 256,
        };
        assert_eq!(resolve_auto_profile(Some(snap)), StorageProfile::Mid);
    }

    #[test]
    fn resolve_1024mb_returns_high() {
        let snap = HardwareSnapshot {
            total_memory_mb: 1024,
        };
        assert_eq!(resolve_auto_profile(Some(snap)), StorageProfile::High);
    }

    #[test]
    fn display_round_trips() {
        for profile in [
            StorageProfile::Auto,
            StorageProfile::High,
            StorageProfile::Mid,
            StorageProfile::Extreme,
        ] {
            let s = profile.to_string();
            assert_eq!(StorageProfile::parse(&s), Some(profile));
        }
    }

    #[test]
    fn explicit_profile_is_clamped_when_hardware_is_too_small() {
        let snap = HardwareSnapshot {
            total_memory_mb: 64,
        };
        assert_eq!(
            resolve_profile_with_hardware_guard(StorageProfile::High, Some(snap)),
            StorageProfile::Extreme
        );
    }

    #[test]
    fn explicit_lower_profile_is_honored_on_strong_hardware() {
        let snap = HardwareSnapshot {
            total_memory_mb: 2048,
        };
        assert_eq!(
            resolve_profile_with_hardware_guard(StorageProfile::Extreme, Some(snap)),
            StorageProfile::Extreme
        );
    }

    #[test]
    fn explicit_profile_is_honored_when_hardware_unknown() {
        assert_eq!(
            resolve_profile_with_hardware_guard(StorageProfile::High, None),
            StorageProfile::High
        );
    }
}
