//! Stable human-readable fragments for infrastructure errors (adapter-local).

pub(crate) const SQLITE_MUTEX_POISONED: &str = "sqlite mutex poisoned";
pub(crate) const SQLITE_BUSY_RETRIES_EXHAUSTED: &str = "sqlite busy: retries exhausted";

pub(crate) const STORED_FIELD_REGION: &str = "region";
pub(crate) const STORED_FIELD_DEVICE_CLASS: &str = "device_class";
