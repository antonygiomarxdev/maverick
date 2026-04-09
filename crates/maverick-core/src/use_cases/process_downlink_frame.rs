use maverick_domain::{Downlink, DownlinkPriority, Eui64, Frequency, SpreadingFactor};

use crate::events::{AuditRecord, EventBus, EventKind, EventSource, EventStatus, SystemEvent};
use crate::kernel::GatewaySelector;
use crate::ports::{AuditLogWriter, DownlinkRepository, GatewayRepository};
use crate::{DomainError, Result};

#[derive(Debug, Clone)]
pub struct DownlinkDraft {
    pub payload: Vec<u8>,
    pub f_port: u8,
    pub dev_eui: Eui64,
    pub frequency: Frequency,
    pub spreading_factor: SpreadingFactor,
    pub timestamp: i64,
    pub frame_counter: u32,
    pub priority: DownlinkPriority,
    pub scheduled_at: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct EnqueueDownlinkCommand {
    pub draft: DownlinkDraft,
    pub gateway_override: Option<Eui64>,
    pub correlation_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnqueueDownlinkOutcome {
    pub downlink_id: i64,
}

pub struct ProcessDownlinkFrameService<R, G, A> {
    repository: R,
    gateways: G,
    audit_log: A,
    event_bus: EventBus,
}

impl<R, G, A> ProcessDownlinkFrameService<R, G, A>
where
    R: DownlinkRepository,
    G: GatewayRepository,
    A: AuditLogWriter,
{
    pub fn new(repository: R, gateways: G, audit_log: A, event_bus: EventBus) -> Self {
        Self {
            repository,
            gateways,
            audit_log,
            event_bus,
        }
    }

    pub async fn enqueue(&self, command: EnqueueDownlinkCommand) -> Result<EnqueueDownlinkOutcome> {
        let gateway_eui = self.resolve_gateway(command.gateway_override).await?;
        let mut downlink = Downlink::new(
            command.draft.payload,
            command.draft.f_port,
            command.draft.dev_eui,
            gateway_eui,
            command.draft.frequency,
            command.draft.spreading_factor,
            command.draft.timestamp,
            command.draft.frame_counter,
        )
        .with_priority(command.draft.priority);
        downlink.scheduled_at = command.draft.scheduled_at;

        let downlink_id = self.repository.enqueue(downlink.clone()).await?;

        let mut audit = AuditRecord::new(
            EventSource::Api,
            "downlink.enqueue",
            "downlink",
            EventStatus::Accepted,
            "downlink queued for scheduling",
        )
        .with_metadata("downlink_id", downlink_id.to_string())
        .with_metadata("dev_eui", downlink.dev_eui.to_string())
        .with_metadata("gateway_eui", downlink.gateway_eui.to_string())
        .with_metadata("f_port", downlink.f_port.to_string())
        .with_metadata("payload_size", downlink.payload.len().to_string());
        audit.entity_id = Some(downlink_id.to_string());
        audit.correlation_id = command.correlation_id.clone();
        self.audit_log.record(audit).await?;

        let mut event = SystemEvent::new(
            EventKind::DownlinkCommand,
            EventSource::Api,
            "downlink.enqueue",
            EventStatus::Accepted,
        )
        .with_entity_id(downlink_id.to_string())
        .with_metadata("dev_eui", downlink.dev_eui.to_string())
        .with_metadata("gateway_eui", downlink.gateway_eui.to_string())
        .with_metadata("f_port", downlink.f_port.to_string())
        .with_metadata("payload_size", downlink.payload.len().to_string());
        if let Some(correlation_id) = command.correlation_id {
            event = event.with_correlation_id(correlation_id);
        }
        self.event_bus.publish(event);

        Ok(EnqueueDownlinkOutcome { downlink_id })
    }

    async fn resolve_gateway(&self, gateway_override: Option<Eui64>) -> Result<Eui64> {
        if let Some(gateway_eui) = gateway_override {
            return Ok(gateway_eui);
        }

        GatewaySelector::new(&self.gateways)
            .select_best()
            .await?
            .map(|gateway| gateway.gateway_eui)
            .ok_or_else(|| DomainError::InvalidState {
                entity: "downlink",
                reason: "no healthy gateways available for automatic selection".to_string(),
            }
            .into())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use std::time::{SystemTime, UNIX_EPOCH};

    use async_trait::async_trait;
    use maverick_domain::{DownlinkPriority, Eui64, Frequency, Gateway, GatewayStatus, SpreadingFactor};

    use super::{DownlinkDraft, EnqueueDownlinkCommand, ProcessDownlinkFrameService};
    use crate::events::{AuditRecord, EventBus};
    use crate::ports::{
        AuditLogWriter, DownlinkRepository, DownlinkState, GatewayRepository, QueuedDownlink,
    };
    use crate::Result;

    #[derive(Clone)]
    struct MockDownlinkRepository {
        next_id: Arc<Mutex<i64>>,
        queued: Arc<Mutex<Vec<maverick_domain::Downlink>>>,
    }

    impl MockDownlinkRepository {
        fn new() -> Self {
            Self {
                next_id: Arc::new(Mutex::new(1)),
                queued: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn first_gateway(&self) -> Option<Eui64> {
            self.queued
                .lock()
                .expect("queued lock")
                .first()
                .map(|value| value.gateway_eui)
        }
    }

    #[async_trait]
    impl DownlinkRepository for MockDownlinkRepository {
        async fn enqueue(&self, downlink: maverick_domain::Downlink) -> Result<i64> {
            self.queued.lock().expect("queued lock").push(downlink);
            let mut next = self.next_id.lock().expect("id lock");
            let id = *next;
            *next += 1;
            Ok(id)
        }

        async fn get_by_id(&self, _id: i64) -> Result<Option<QueuedDownlink>> {
            Ok(None)
        }

        async fn list_by_dev_eui(
            &self,
            _dev_eui: Eui64,
            _state: Option<DownlinkState>,
            _limit: usize,
        ) -> Result<Vec<QueuedDownlink>> {
            Ok(Vec::new())
        }

        async fn list_pending(&self, _limit: usize) -> Result<Vec<QueuedDownlink>> {
            Ok(Vec::new())
        }

        async fn mark_scheduled(&self, _id: i64, _scheduled_at: i64) -> Result<()> {
            Ok(())
        }

        async fn mark_sent(&self, _id: i64, _sent_at: i64) -> Result<()> {
            Ok(())
        }

        async fn mark_retry(&self, _id: i64, _retry_at: i64, _reason: &str) -> Result<()> {
            Ok(())
        }

        async fn mark_retry_with_gateway(
            &self,
            _id: i64,
            _retry_at: i64,
            _gateway_eui: Eui64,
            _reason: &str,
        ) -> Result<()> {
            Ok(())
        }

        async fn mark_failed(&self, _id: i64, _reason: &str) -> Result<()> {
            Ok(())
        }
    }

    #[derive(Clone)]
    struct MockGatewayRepository {
        gateways: Arc<Mutex<Vec<Gateway>>>,
    }

    impl MockGatewayRepository {
        fn new(gateways: Vec<Gateway>) -> Self {
            Self {
                gateways: Arc::new(Mutex::new(gateways)),
            }
        }
    }

    #[async_trait]
    impl GatewayRepository for MockGatewayRepository {
        async fn create(&self, gateway: Gateway) -> Result<Gateway> {
            self.gateways
                .lock()
                .expect("gateways lock")
                .push(gateway.clone());
            Ok(gateway)
        }

        async fn update(&self, gateway: Gateway) -> Result<Gateway> {
            Ok(gateway)
        }

        async fn delete(&self, _gateway_eui: Eui64) -> Result<()> {
            Ok(())
        }

        async fn get_by_gateway_eui(&self, gateway_eui: Eui64) -> Result<Option<Gateway>> {
            Ok(self
                .gateways
                .lock()
                .expect("gateways lock")
                .iter()
                .find(|value| value.gateway_eui == gateway_eui)
                .cloned())
        }

        async fn list(&self, status: Option<GatewayStatus>) -> Result<Vec<Gateway>> {
            let data = self.gateways.lock().expect("gateways lock");
            Ok(data
                .iter()
                .filter(|value| status.map(|item| value.status == item).unwrap_or(true))
                .cloned()
                .collect())
        }

        async fn list_healthy(&self) -> Result<Vec<Gateway>> {
            self.list(Some(GatewayStatus::Online)).await
        }
    }

    #[derive(Clone)]
    struct MockAuditLogWriter;

    #[async_trait]
    impl AuditLogWriter for MockAuditLogWriter {
        async fn record(&self, _record: AuditRecord) -> Result<()> {
            Ok(())
        }
    }

    fn unix_timestamp() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs() as i64)
            .unwrap_or_default()
    }

    fn draft() -> DownlinkDraft {
        DownlinkDraft {
            payload: vec![0x01, 0x02],
            f_port: 10,
            dev_eui: Eui64::from([0x10; 8]),
            frequency: Frequency::new(868_100_000),
            spreading_factor: SpreadingFactor::new(7).expect("valid sf"),
            timestamp: unix_timestamp(),
            frame_counter: 1,
            priority: DownlinkPriority::High,
            scheduled_at: None,
        }
    }

    #[tokio::test]
    async fn explicit_override_uses_command_gateway() {
        let repository = MockDownlinkRepository::new();
        let override_gateway = Eui64::from([0xAB; 8]);
        let selector_candidate = Eui64::from([0xCD; 8]);
        let mut online = Gateway::new(selector_candidate);
        online.status = GatewayStatus::Online;
        online.last_seen = Some(unix_timestamp());
        let gateways = MockGatewayRepository::new(vec![online]);

        let service = ProcessDownlinkFrameService::new(
            repository.clone(),
            gateways,
            MockAuditLogWriter,
            EventBus::new(8),
        );

        service
            .enqueue(EnqueueDownlinkCommand {
                draft: draft(),
                gateway_override: Some(override_gateway),
                correlation_id: Some("test-correlation".to_string()),
            })
            .await
            .expect("enqueue succeeds");

        assert_eq!(repository.first_gateway(), Some(override_gateway));
    }

    #[tokio::test]
    async fn auto_selection_uses_online_gateway() {
        let repository = MockDownlinkRepository::new();
        let preferred_gateway = Eui64::from([0x11; 8]);
        let secondary_gateway = Eui64::from([0x22; 8]);

        let mut preferred = Gateway::new(preferred_gateway);
        preferred.status = GatewayStatus::Online;
        preferred.last_seen = Some(unix_timestamp());
        preferred.tx_frequency = Some(868_100_000);

        let mut secondary = Gateway::new(secondary_gateway);
        secondary.status = GatewayStatus::Online;
        secondary.last_seen = Some(unix_timestamp() - 60);

        let gateways = MockGatewayRepository::new(vec![secondary, preferred]);

        let service = ProcessDownlinkFrameService::new(
            repository.clone(),
            gateways,
            MockAuditLogWriter,
            EventBus::new(8),
        );

        service
            .enqueue(EnqueueDownlinkCommand {
                draft: draft(),
                gateway_override: None,
                correlation_id: None,
            })
            .await
            .expect("enqueue succeeds");

        assert_eq!(repository.first_gateway(), Some(preferred_gateway));
    }

    #[tokio::test]
    async fn enqueue_fails_without_healthy_gateways() {
        let repository = MockDownlinkRepository::new();
        let mut offline = Gateway::new(Eui64::from([0x33; 8]));
        offline.status = GatewayStatus::Offline;
        let gateways = MockGatewayRepository::new(vec![offline]);

        let service = ProcessDownlinkFrameService::new(
            repository,
            gateways,
            MockAuditLogWriter,
            EventBus::new(8),
        );

        let error = service
            .enqueue(EnqueueDownlinkCommand {
                draft: draft(),
                gateway_override: None,
                correlation_id: None,
            })
            .await
            .expect_err("must fail without healthy gateways");

        assert!(
            error
                .to_string()
                .contains("no healthy gateways available for automatic selection")
        );
    }
}
