//! Loop timeout / throughput presets derived from host memory hints.

use crate::config::TuiConfig;

pub(crate) fn apply_suggested_profile(cfg: &mut TuiConfig, total_memory_bytes: Option<u64>) {
    let profile = total_memory_bytes
        .map(suggested_profile_from_memory)
        .unwrap_or("balanced")
        .to_string();
    apply_profile_defaults(cfg, &profile);
}

/// When `profile` is `auto`, pass host memory from `maverick-edge probe` if available (`None` => balanced).
pub(crate) fn apply_profile_by_name(
    cfg: &mut TuiConfig,
    profile: &str,
    auto_memory_bytes: Option<u64>,
) -> Result<(), String> {
    if profile.eq_ignore_ascii_case("auto") {
        apply_suggested_profile(cfg, auto_memory_bytes);
        return Ok(());
    }

    let normalized = normalize_profile(profile).ok_or_else(|| {
        format!("invalid profile '{profile}'; expected auto|constrained|balanced|high-capacity")
    })?;
    apply_profile_defaults(cfg, normalized);
    Ok(())
}

pub(crate) fn apply_profile_defaults(cfg: &mut TuiConfig, profile: &str) {
    match normalize_profile(profile).unwrap_or("balanced") {
        "constrained" => {
            cfg.loop_read_timeout_ms = 1_500;
            cfg.loop_max_messages = 0;
        }
        "balanced" => {
            cfg.loop_read_timeout_ms = 1_000;
            cfg.loop_max_messages = 0;
        }
        "high-capacity" => {
            cfg.loop_read_timeout_ms = 700;
            cfg.loop_max_messages = 0;
        }
        _ => {}
    }
}

pub(crate) fn normalize_profile(profile: &str) -> Option<&'static str> {
    let p = profile.to_ascii_lowercase();
    match p.as_str() {
        "constrained" => Some("constrained"),
        "balanced" => Some("balanced"),
        "high-capacity" | "high_capacity" | "highcapacity" => Some("high-capacity"),
        _ => None,
    }
}

pub(crate) fn suggested_profile_from_memory(total_memory_bytes: u64) -> &'static str {
    const MIB: u64 = 1024 * 1024;
    const MEMORY_BYTES_512_MIB: u64 = 512 * MIB;
    const MEMORY_BYTES_2_GIB: u64 = 2 * 1024 * MIB;

    if total_memory_bytes < MEMORY_BYTES_512_MIB {
        "constrained"
    } else if total_memory_bytes < MEMORY_BYTES_2_GIB {
        "balanced"
    } else {
        "high-capacity"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TuiConfig;

    #[test]
    fn suggested_profile_buckets_match_thresholds() {
        assert_eq!(suggested_profile_from_memory(0), "constrained");
        assert_eq!(suggested_profile_from_memory(700 * 1024 * 1024), "balanced");
        assert_eq!(
            suggested_profile_from_memory(4 * 1024 * 1024 * 1024),
            "high-capacity"
        );
    }

    #[test]
    fn apply_profile_defaults_sets_expected_values() {
        let mut cfg = TuiConfig::default();

        apply_profile_defaults(&mut cfg, "constrained");
        assert_eq!(cfg.loop_read_timeout_ms, 1_500);
        assert_eq!(cfg.loop_max_messages, 0);

        apply_profile_defaults(&mut cfg, "balanced");
        assert_eq!(cfg.loop_read_timeout_ms, 1_000);
        assert_eq!(cfg.loop_max_messages, 0);

        apply_profile_defaults(&mut cfg, "high-capacity");
        assert_eq!(cfg.loop_read_timeout_ms, 700);
        assert_eq!(cfg.loop_max_messages, 0);
    }
}
