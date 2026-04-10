use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::StoragePressureLevel;

/// Point-in-time storage pressure for operators and health aggregation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoragePressureSnapshot {
    pub level: StoragePressureLevel,
    /// On-disk size of the SQLite database file (best-effort).
    pub db_bytes: u64,
    /// Total disk capacity used for ratio thresholds, when configured.
    pub total_disk_bytes: Option<u64>,
    pub detail: Option<String>,
}

/// Implemented by concrete persistence adapters; core stays I/O-free.
#[async_trait]
pub trait StoragePressureSource: Send + Sync {
    async fn pressure_snapshot(&self) -> StoragePressureSnapshot;
}
