pub mod dto;
pub mod routes;

use axum::Router;
use std::sync::Arc;

use crate::adapters::persistence::SqliteUplinkRepository;
use crate::config::RuntimeConfig;
use crate::db::Database;
use crate::events::EventBus;
use crate::kernel::KernelServices;
use crate::ports::UplinkRepository;

#[derive(Clone)]
pub struct AppState<D: Database> {
    pub services: Arc<KernelServices<D>>,
}

impl<D: Database + 'static> AppState<D> {
    pub fn new(services: KernelServices<D>) -> Self {
        Self {
            services: Arc::new(services),
        }
    }

    pub fn from_parts(db: D, config: RuntimeConfig, version: &'static str) -> Self {
        let db = Arc::new(db);
        let event_bus = EventBus::new(config.event_bus_capacity);
        let uplink_repo: Arc<dyn UplinkRepository + Send + Sync> =
            Arc::new(SqliteUplinkRepository::new(db.clone()));
        Self::new(KernelServices::new(
            db,
            config,
            version,
            event_bus,
            uplink_repo,
        ))
    }
}

pub fn create_app<D: Database + Clone + Send + Sync + 'static>(state: AppState<D>) -> Router {
    Router::new()
        .nest("/api/v1", routes::routes())
        .with_state(state)
}
