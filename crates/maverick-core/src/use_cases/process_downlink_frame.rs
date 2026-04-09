use maverick_domain::Downlink;

use crate::events::{AuditRecord, EventBus, EventKind, EventSource, EventStatus, SystemEvent};
use crate::ports::{AuditLogWriter, DownlinkRepository};
use crate::Result;

#[derive(Debug, Clone)]
pub struct EnqueueDownlinkCommand {
    pub downlink: Downlink,
    pub correlation_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnqueueDownlinkOutcome {
    pub downlink_id: i64,
}

pub struct ProcessDownlinkFrameService<R, A> {
    repository: R,
    audit_log: A,
    event_bus: EventBus,
}

impl<R, A> ProcessDownlinkFrameService<R, A>
where
    R: DownlinkRepository,
    A: AuditLogWriter,
{
    pub fn new(repository: R, audit_log: A, event_bus: EventBus) -> Self {
        Self {
            repository,
            audit_log,
            event_bus,
        }
    }

    pub async fn enqueue(&self, command: EnqueueDownlinkCommand) -> Result<EnqueueDownlinkOutcome> {
        let downlink_id = self.repository.enqueue(command.downlink.clone()).await?;

        let mut audit = AuditRecord::new(
            EventSource::Api,
            "downlink.enqueue",
            "downlink",
            EventStatus::Accepted,
            "downlink queued for scheduling",
        )
        .with_metadata("downlink_id", downlink_id.to_string())
        .with_metadata("dev_eui", command.downlink.dev_eui.to_string())
        .with_metadata("gateway_eui", command.downlink.gateway_eui.to_string())
        .with_metadata("f_port", command.downlink.f_port.to_string())
        .with_metadata("payload_size", command.downlink.payload.len().to_string());
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
        .with_metadata("dev_eui", command.downlink.dev_eui.to_string())
        .with_metadata("gateway_eui", command.downlink.gateway_eui.to_string())
        .with_metadata("f_port", command.downlink.f_port.to_string())
        .with_metadata("payload_size", command.downlink.payload.len().to_string());
        if let Some(correlation_id) = command.correlation_id {
            event = event.with_correlation_id(correlation_id);
        }
        self.event_bus.publish(event);

        Ok(EnqueueDownlinkOutcome { downlink_id })
    }
}
