use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use maverick_domain::{Downlink, Eui64};

use crate::events::{AuditRecord, EventBus, EventKind, EventSource, EventStatus, SystemEvent};
use crate::kernel::GatewaySelector;
use crate::ports::{
    AuditLogWriter, DownlinkRepository, DownlinkState, GatewayRepository, QueuedDownlink,
};
use crate::Result;

#[async_trait]
pub trait DownlinkSender: Send + Sync {
    async fn send(&self, downlink: &Downlink) -> Result<()>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeliveryConfig {
    pub fetch_limit: usize,
    pub schedule_delay_secs: i64,
    pub retry_delay_secs: i64,
    pub max_attempts: u32,
}

impl Default for DeliveryConfig {
    fn default() -> Self {
        Self {
            fetch_limit: 128,
            schedule_delay_secs: 2,
            retry_delay_secs: 3,
            max_attempts: 3,
        }
    }
}

pub struct NoopDownlinkSender;

#[async_trait]
impl DownlinkSender for NoopDownlinkSender {
    async fn send(&self, _downlink: &Downlink) -> Result<()> {
        Ok(())
    }
}

pub struct DownlinkDeliveryService<R, G, A, S> {
    repository: R,
    gateways: G,
    audit_log: A,
    event_bus: EventBus,
    sender: Arc<S>,
    config: DeliveryConfig,
}

impl<R, G, A, S> DownlinkDeliveryService<R, G, A, S>
where
    R: DownlinkRepository + 'static,
    G: GatewayRepository + 'static,
    A: AuditLogWriter + 'static,
    S: DownlinkSender + 'static,
{
    pub fn new(
        repository: R,
        gateways: G,
        audit_log: A,
        event_bus: EventBus,
        sender: Arc<S>,
        config: DeliveryConfig,
    ) -> Self {
        Self {
            repository,
            gateways,
            audit_log,
            event_bus,
            sender,
            config,
        }
    }

    pub async fn process_once(&self) -> Result<usize> {
        let pending = self
            .repository
            .list_pending(self.config.fetch_limit)
            .await?;
        if pending.is_empty() {
            return Ok(0);
        }

        let now = unix_timestamp();
        let mut processed = 0usize;

        for item in pending {
            match item.state {
                DownlinkState::Queued => {
                    let scheduled_at = item
                        .downlink
                        .scheduled_at
                        .unwrap_or(now + self.config.schedule_delay_secs);
                    self.repository
                        .mark_scheduled(item.id, scheduled_at)
                        .await?;
                    self.record_event(
                        item.id,
                        "downlink.scheduled",
                        EventStatus::Accepted,
                        Some(item.downlink.dev_eui.to_string()),
                        None,
                    )
                    .await?;
                    processed += 1;
                }
                DownlinkState::Scheduled => {
                    if item.downlink.scheduled_at.unwrap_or(now) > now {
                        continue;
                    }

                    if let Err(error) = self.sender.send(&item.downlink).await {
                        self.handle_send_failure(item, now, &error.to_string())
                            .await?;
                    } else {
                        self.repository.mark_sent(item.id, now).await?;
                        self.record_event(
                            item.id,
                            "downlink.sent",
                            EventStatus::Succeeded,
                            Some(item.downlink.dev_eui.to_string()),
                            None,
                        )
                        .await?;
                    }

                    processed += 1;
                }
                DownlinkState::Sent | DownlinkState::Failed => {}
            }
        }

        Ok(processed)
    }

    pub fn spawn_delivery_loop(self, interval: Duration) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            loop {
                ticker.tick().await;
                if let Err(error) = self.process_once().await {
                    tracing::warn!(error = %error, "downlink delivery loop failed");
                }
            }
        })
    }

    async fn handle_send_failure(
        &self,
        item: QueuedDownlink,
        now: i64,
        reason: &str,
    ) -> Result<()> {
        let next_attempt = item.attempt_count + 1;
        if next_attempt >= self.config.max_attempts {
            self.repository.mark_failed(item.id, reason).await?;
            self.record_event(
                item.id,
                "downlink.failed",
                EventStatus::Failed,
                Some(item.downlink.dev_eui.to_string()),
                Some(reason.to_string()),
            )
            .await?;
            return Ok(());
        }

        let retry_at = now + self.config.retry_delay_secs;
        if let Some(alternate_gateway) = self.next_gateway_candidate(item.downlink.gateway_eui).await? {
            self.repository
                .mark_retry_with_gateway(item.id, retry_at, alternate_gateway, reason)
                .await?;
            self.record_event(
                item.id,
                "downlink.failover",
                EventStatus::Rejected,
                Some(item.downlink.dev_eui.to_string()),
                Some(format!(
                    "gateway {} -> {}: {reason}",
                    item.downlink.gateway_eui,
                    alternate_gateway
                )),
            )
            .await?;
        } else {
            self.repository
                .mark_retry(item.id, retry_at, reason)
                .await?;
        }
        self.record_event(
            item.id,
            "downlink.retry",
            EventStatus::Rejected,
            Some(item.downlink.dev_eui.to_string()),
            Some(reason.to_string()),
        )
        .await?;
        Ok(())
    }

    async fn next_gateway_candidate(&self, current_gateway: Eui64) -> Result<Option<Eui64>> {
        let selector = GatewaySelector::new(&self.gateways);
        let candidate = selector
            .healthy_candidates()
            .await?
            .into_iter()
            .find(|candidate| candidate.gateway_eui != current_gateway)
            .map(|candidate| candidate.gateway_eui);
        Ok(candidate)
    }

    async fn record_event(
        &self,
        downlink_id: i64,
        operation: &str,
        status: EventStatus,
        entity_id: Option<String>,
        reason: Option<String>,
    ) -> Result<()> {
        let mut audit = AuditRecord::new(
            EventSource::System,
            operation,
            "downlink",
            status.clone(),
            operation,
        )
        .with_metadata("downlink_id", downlink_id.to_string());
        audit.entity_id = entity_id.clone();
        audit.reason_code = reason.clone();
        self.audit_log.record(audit).await?;

        let mut event = SystemEvent::new(
            EventKind::DownlinkCommand,
            EventSource::System,
            operation,
            status,
        )
        .with_entity_id(downlink_id.to_string());
        if let Some(entity_id) = entity_id {
            event = event.with_metadata("dev_eui", entity_id);
        }
        if let Some(reason) = reason {
            event = event.with_reason_code(reason);
        }
        self.event_bus.publish(event);
        Ok(())
    }
}

fn unix_timestamp() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use maverick_domain::{Downlink, Eui64, Frequency, Gateway, GatewayStatus, SpreadingFactor};

    use super::{DeliveryConfig, DownlinkDeliveryService, DownlinkSender};
    use crate::adapters::persistence::{
        SqliteAuditLogWriter, SqliteDownlinkRepository, SqliteGatewayRepository,
    };
    use crate::db::SqliteDb;
    use crate::events::EventBus;
    use crate::ports::{DownlinkRepository, DownlinkState, GatewayRepository};

    struct FailingSender;

    #[async_trait]
    impl DownlinkSender for FailingSender {
        async fn send(&self, _downlink: &Downlink) -> crate::Result<()> {
            Err(crate::AppError::Event("gateway send failed".to_string()))
        }
    }

    struct FailFirstGatewaySender {
        failing_gateway: Eui64,
    }

    #[async_trait]
    impl DownlinkSender for FailFirstGatewaySender {
        async fn send(&self, downlink: &Downlink) -> crate::Result<()> {
            if downlink.gateway_eui == self.failing_gateway {
                Err(crate::AppError::Event("gateway send failed".to_string()))
            } else {
                Ok(())
            }
        }
    }

    fn sample_downlink() -> Downlink {
        let mut downlink = Downlink::new(
            vec![0x01],
            10,
            Eui64::from([1, 2, 3, 4, 5, 6, 7, 8]),
            Eui64::from([8, 7, 6, 5, 4, 3, 2, 1]),
            Frequency::new(868_100_000),
            SpreadingFactor::new(7).expect("spreading factor must be valid"),
            0,
            1,
        );
        downlink.scheduled_at = Some(0);
        downlink
    }

    #[tokio::test]
    async fn worker_marks_scheduled_and_sent_with_success_sender() {
        let db = Arc::new(SqliteDb::in_memory().await.expect("db must open"));
        let repository = SqliteDownlinkRepository::new(db.clone());
        let gateways = SqliteGatewayRepository::new(db.clone());
        let audit = SqliteAuditLogWriter::new(db);
        let event_bus = EventBus::new(16);

        let id = repository
            .enqueue(sample_downlink())
            .await
            .expect("enqueue must succeed");

        let service = DownlinkDeliveryService::new(
            repository,
            gateways,
            audit,
            event_bus,
            Arc::new(super::NoopDownlinkSender),
            DeliveryConfig {
                schedule_delay_secs: 0,
                ..DeliveryConfig::default()
            },
        );

        service.process_once().await.expect("first pass must work");
        service.process_once().await.expect("second pass must work");

        let current = service
            .repository
            .get_by_id(id)
            .await
            .expect("query must succeed")
            .expect("downlink must exist");
        assert_eq!(current.state, DownlinkState::Sent);
    }

    #[tokio::test]
    async fn worker_retries_and_then_marks_failed() {
        let db = Arc::new(SqliteDb::in_memory().await.expect("db must open"));
        let repository = SqliteDownlinkRepository::new(db.clone());
        let gateways = SqliteGatewayRepository::new(db.clone());
        let audit = SqliteAuditLogWriter::new(db);
        let event_bus = EventBus::new(16);

        let id = repository
            .enqueue(sample_downlink())
            .await
            .expect("enqueue must succeed");

        let service = DownlinkDeliveryService::new(
            repository,
            gateways,
            audit,
            event_bus,
            Arc::new(FailingSender),
            DeliveryConfig {
                schedule_delay_secs: 0,
                retry_delay_secs: 0,
                max_attempts: 2,
                ..DeliveryConfig::default()
            },
        );

        service.process_once().await.expect("schedule must work");
        service
            .process_once()
            .await
            .expect("first send fail must work");
        service
            .process_once()
            .await
            .expect("retry scheduling must work");
        service
            .process_once()
            .await
            .expect("second send fail must work");

        let current = service
            .repository
            .get_by_id(id)
            .await
            .expect("query must succeed")
            .expect("downlink must exist");
        assert_eq!(current.state, DownlinkState::Failed);
        assert!(current.attempt_count >= 2);
    }

    #[tokio::test]
    async fn worker_failover_switches_gateway_before_next_retry() {
        let db = Arc::new(SqliteDb::in_memory().await.expect("db must open"));
        let repository = SqliteDownlinkRepository::new(db.clone());
        let gateways = SqliteGatewayRepository::new(db.clone());
        let audit = SqliteAuditLogWriter::new(db.clone());
        let event_bus = EventBus::new(16);

        let first_gateway = Eui64::from([8, 7, 6, 5, 4, 3, 2, 1]);
        let second_gateway = Eui64::from([9, 7, 6, 5, 4, 3, 2, 1]);

        let mut primary = Gateway::new(first_gateway);
        primary.status = GatewayStatus::Online;
        primary.last_seen = Some(super::unix_timestamp());
        primary.tx_frequency = Some(868_100_000);
        gateways.create(primary).await.expect("primary must persist");

        let mut alternate = Gateway::new(second_gateway);
        alternate.status = GatewayStatus::Online;
        alternate.last_seen = Some(super::unix_timestamp() - 1);
        alternate.tx_frequency = Some(868_100_000);
        gateways.create(alternate).await.expect("alternate must persist");

        let id = repository
            .enqueue(sample_downlink())
            .await
            .expect("enqueue must succeed");

        let service = DownlinkDeliveryService::new(
            repository,
            gateways,
            audit,
            event_bus,
            Arc::new(FailFirstGatewaySender {
                failing_gateway: first_gateway,
            }),
            DeliveryConfig {
                schedule_delay_secs: 0,
                retry_delay_secs: 0,
                max_attempts: 3,
                ..DeliveryConfig::default()
            },
        );

        service.process_once().await.expect("schedule must work");
        service.process_once().await.expect("first send fail must work");

        let retried = service
            .repository
            .get_by_id(id)
            .await
            .expect("query must succeed")
            .expect("downlink must exist");
        assert_eq!(retried.state, DownlinkState::Queued);
        assert_eq!(retried.downlink.gateway_eui, second_gateway);

        service.process_once().await.expect("reschedule must work");
        service.process_once().await.expect("second send must succeed");

        let current = service
            .repository
            .get_by_id(id)
            .await
            .expect("query must succeed")
            .expect("downlink must exist");
        assert_eq!(current.state, DownlinkState::Sent);
        assert_eq!(current.downlink.gateway_eui, second_gateway);
    }
}
