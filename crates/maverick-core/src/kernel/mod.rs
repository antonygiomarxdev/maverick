mod gateway_selector;

use std::sync::Arc;

use crate::adapters::persistence::{
    SqliteAuditLogWriter, SqliteDeviceRepository, SqliteDownlinkRepository,
    SqliteGatewayRepository, SqliteSessionRepository,
};
use crate::config::RuntimeConfig;
use crate::db::Database;
use crate::events::EventBus;
use crate::ports::UplinkRepository;
use crate::use_cases::{
    DeviceManagementService, IngestUplinkService, ProcessDownlinkFrameService,
    ProcessUplinkFrameService,
};
pub use gateway_selector::{GatewayCandidateScore, GatewaySelector};

#[derive(Clone)]
pub struct KernelServices<D: Database> {
    pub db: Arc<D>,
    pub config: RuntimeConfig,
    pub event_bus: EventBus,
    pub version: &'static str,
    pub uplink_repo: Arc<dyn UplinkRepository + Send + Sync>,
    pub device_repo: SqliteDeviceRepository<D>,
    pub gateway_repo: SqliteGatewayRepository<D>,
    pub downlink_repo: SqliteDownlinkRepository<D>,
    pub session_repo: SqliteSessionRepository<D>,
    pub audit_log: SqliteAuditLogWriter<D>,
}

impl<D: Database> KernelServices<D> {
    pub fn new(
        db: Arc<D>,
        config: RuntimeConfig,
        version: &'static str,
        event_bus: EventBus,
        uplink_repo: Arc<dyn UplinkRepository + Send + Sync>,
    ) -> Self {
        let device_repo = SqliteDeviceRepository::new(db.clone());
        let gateway_repo = SqliteGatewayRepository::new(db.clone());
        let downlink_repo = SqliteDownlinkRepository::new(db.clone());
        let session_repo = SqliteSessionRepository::new(db.clone());
        let audit_log = SqliteAuditLogWriter::new(db.clone());
        Self {
            db,
            config,
            event_bus,
            version,
            uplink_repo,
            device_repo,
            gateway_repo,
            downlink_repo,
            session_repo,
            audit_log,
        }
    }

    pub fn device_service(
        &self,
    ) -> DeviceManagementService<&SqliteDeviceRepository<D>, &SqliteAuditLogWriter<D>> {
        DeviceManagementService::new(&self.device_repo, &self.audit_log, self.event_bus.clone())
    }

    pub fn downlink_service(
        &self,
    ) -> ProcessDownlinkFrameService<
        &SqliteDownlinkRepository<D>,
        &SqliteGatewayRepository<D>,
        &SqliteAuditLogWriter<D>,
    > {
        ProcessDownlinkFrameService::new(
            &self.downlink_repo,
            &self.gateway_repo,
            &self.audit_log,
            self.event_bus.clone(),
        )
    }

    pub fn gateway_selector(&self) -> GatewaySelector<&SqliteGatewayRepository<D>> {
        GatewaySelector::new(&self.gateway_repo)
    }

    pub fn ingest_uplink_service(
        &self,
    ) -> IngestUplinkService<
        Arc<dyn UplinkRepository + Send + Sync>,
        &SqliteGatewayRepository<D>,
        &SqliteAuditLogWriter<D>,
    > {
        IngestUplinkService::new(
            Arc::clone(&self.uplink_repo),
            &self.gateway_repo,
            &self.audit_log,
            self.event_bus.clone(),
        )
    }

    pub fn process_uplink_frame_service(
        &self,
    ) -> ProcessUplinkFrameService<
        &SqliteSessionRepository<D>,
        &SqliteAuditLogWriter<D>,
        &SqliteDeviceRepository<D>,
    > {
        ProcessUplinkFrameService::new(
            &self.session_repo,
            &self.audit_log,
            self.event_bus.clone(),
            &self.device_repo,
        )
    }
}
