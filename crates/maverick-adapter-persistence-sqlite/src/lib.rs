//! Durable local persistence via SQLite implementing `maverick-core` ports.
//!
//! Retention: telemetry (uplinks), operational (audit), critical (sessions) with optional
//! circular trimming when disk pressure reaches configured hard-limit semantics.

mod diag;
mod limits;
mod persisted_device_class;
mod persistence;
pub mod schema;
mod sqlite_op;

pub use persistence::{
    LnsApplicationRow, LnsAutoprovisionMeta, LnsDeviceListRow, LnsPendingRow, SqlitePersistence,
    SqlitePersistenceOptions,
};
