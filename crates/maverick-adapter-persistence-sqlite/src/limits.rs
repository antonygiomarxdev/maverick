//! Numeric policy thresholds for disk pressure, tier fill, and hard-trim batches (adapter-local).

/// When DB file size / total disk reaches this ratio, treat as hard-limit pressure (with disk hint).
pub const DISK_RATIO_HARD_LIMIT_ENTER: f64 = 0.98;

/// Target ratio after circular trim rounds (best-effort; file may not shrink without VACUUM).
pub const DISK_RATIO_HARD_LIMIT_TARGET: f64 = 0.92;

/// Tier fill ratio (records / tier cap) at or above this maps to elevated pressure.
pub const TIER_FILL_ELEVATED_RATIO: f64 = 0.9;

/// Tier fill at or above this ratio maps to critical pressure (at/over configured caps).
pub const TIER_FILL_CRITICAL_RATIO: f64 = 1.0;

/// Maximum rounds of batched deletes under hard-limit circular mode.
pub const HARD_TRIM_MAX_ROUNDS: u32 = 8;

pub const HARD_TRIM_UPLINK_BATCH: i64 = 500;
pub const HARD_TRIM_AUDIT_BATCH: i64 = 500;
pub const HARD_TRIM_SESSION_BATCH: i64 = 50;

/// Base backoff step for SQLite busy retries: `BASE_MS * (attempt + 1)`.
pub const BUSY_RETRY_BACKOFF_BASE_MS: u64 = 10;

/// Expected DevEUI length in bytes when persisting sessions.
pub const DEV_EUI_BYTE_LEN: usize = 8;
