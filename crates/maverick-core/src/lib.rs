//! Application kernel: use cases, ports (`traits`), protocol capability modules, storage policy.
//!
//! Must not depend on HTTP, concrete DB, or socket crates.

pub mod error;
pub mod health;
pub mod ports;
pub mod protocol;
pub mod storage;
pub mod use_cases;

pub use error::AppError;
pub use health::{ComponentHealth, HealthState, HealthStatus};
pub use protocol::{ProtocolCapability, ProtocolContext, ProtocolDecision};
pub use storage::{
    HybridRetentionDefaults, InstallProfile, RetentionTier, StoragePolicy, StoragePressureLevel,
};
