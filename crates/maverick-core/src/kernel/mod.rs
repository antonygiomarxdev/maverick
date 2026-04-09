use std::sync::Arc;

use crate::config::RuntimeConfig;
use crate::db::Database;
use crate::events::EventBus;
use crate::ports::UplinkRepository;

#[derive(Clone)]
pub struct KernelServices<D: Database> {
    pub db: Arc<D>,
    pub config: RuntimeConfig,
    pub event_bus: EventBus,
    pub version: &'static str,
    pub uplink_repo: Arc<dyn UplinkRepository + Send + Sync>,
}

impl<D: Database> KernelServices<D> {
    pub fn new(
        db: Arc<D>,
        config: RuntimeConfig,
        version: &'static str,
        event_bus: EventBus,
        uplink_repo: Arc<dyn UplinkRepository + Send + Sync>,
    ) -> Self {
        Self {
            db,
            config,
            event_bus,
            version,
            uplink_repo,
        }
    }
}
