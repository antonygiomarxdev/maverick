use std::sync::Arc;
use std::time::Duration;

use crate::api::AppState;
use crate::config::RuntimeConfig;
use crate::db::Database;
use crate::events::EventBus;
use crate::ingester::run_udp_ingester;
use crate::kernel::KernelServices;
use crate::ports::UplinkRepository;
use crate::storage_profile::StorageProfile;
use crate::use_cases::{
    DeliveryConfig, DownlinkDeliveryService, NoopDownlinkSender, RetentionService,
};
use crate::Result;

pub fn build_app_state<D: Database + 'static>(
    db: Arc<D>,
    config: RuntimeConfig,
    version: &'static str,
    event_bus: EventBus,
    uplink_repo: Arc<dyn UplinkRepository + Send + Sync>,
) -> AppState<D> {
    AppState::new(KernelServices::new(db, config, version, event_bus, uplink_repo))
}

pub fn spawn_runtime_tasks<D: Database + Clone + Send + Sync + 'static>(
    state: &AppState<D>,
    resolved_storage_profile: StorageProfile,
) -> tokio::task::JoinHandle<Result<()>> {
    if resolved_storage_profile != StorageProfile::Extreme {
        let retention = RetentionService::new(
            state.services.db.clone(),
            state.services.config.storage_limits.clone(),
            state.services.event_bus.clone(),
        );
        tokio::spawn(async move { retention.run_forever().await });
    }

    let downlink_delivery = DownlinkDeliveryService::new(
        state.services.downlink_repo.clone(),
        state.services.gateway_repo.clone(),
        state.services.audit_log.clone(),
        state.services.event_bus.clone(),
        Arc::new(NoopDownlinkSender),
        DeliveryConfig::default(),
    );
    downlink_delivery.spawn_delivery_loop(Duration::from_secs(1));

    run_udp_ingester(state.services.clone(), state.services.config.udp_max_datagram_size)
}