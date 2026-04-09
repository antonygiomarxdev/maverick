use maverick_domain::{Gateway, GatewayStatus, UplinkFrame};

use crate::events::{AuditRecord, EventBus, EventKind, EventSource, EventStatus, SystemEvent};
use crate::ports::{AuditLogWriter, GatewayRepository, UplinkRepository};
use crate::Result;

#[derive(Debug, Clone)]
pub struct IngestUplinkCommand {
    pub uplink: UplinkFrame,
    pub correlation_id: Option<String>,
}

pub struct IngestUplinkService<U, G, A> {
    uplink_repository: U,
    gateway_repository: G,
    audit_log: A,
    event_bus: EventBus,
}

impl<U, G, A> IngestUplinkService<U, G, A>
where
    U: UplinkRepository,
    G: GatewayRepository,
    A: AuditLogWriter,
{
    pub fn new(
        uplink_repository: U,
        gateway_repository: G,
        audit_log: A,
        event_bus: EventBus,
    ) -> Self {
        Self {
            uplink_repository,
            gateway_repository,
            audit_log,
            event_bus,
        }
    }

    pub async fn ingest(&self, command: IngestUplinkCommand) -> Result<()> {
        let gateway_eui = command.uplink.gateway_eui.to_string();

        let mut gateway = self
            .gateway_repository
            .get_by_gateway_eui(command.uplink.gateway_eui)
            .await?
            .unwrap_or_else(|| Gateway::new(command.uplink.gateway_eui));
        gateway.status = GatewayStatus::Online;
        gateway.last_seen = Some(command.uplink.timestamp);
        gateway.tx_frequency = Some(command.uplink.frequency.as_hz());

        self.gateway_repository.upsert(gateway).await?;
        self.uplink_repository
            .append(command.uplink.clone())
            .await?;

        let mut audit = AuditRecord::new(
            EventSource::Udp,
            "udp.push_data.accepted",
            "uplink_frame",
            EventStatus::Succeeded,
            "semtech udp payload ingested",
        );
        audit.entity_id = Some(gateway_eui.clone());
        audit.correlation_id = command.correlation_id.clone();
        audit = audit
            .with_metadata("gateway_eui", gateway_eui.clone())
            .with_metadata("payload_size", command.uplink.payload.len().to_string())
            .with_metadata("frequency_hz", command.uplink.frequency.as_hz().to_string());
        self.audit_log.record(audit).await?;

        let mut gateway_event = SystemEvent::new(
            EventKind::GatewayObservation,
            EventSource::Udp,
            "gateway.observed",
            EventStatus::Succeeded,
        )
        .with_entity_id(gateway_eui.clone())
        .with_metadata("status", "Online")
        .with_metadata("frequency_hz", command.uplink.frequency.as_hz().to_string());
        if let Some(correlation_id) = command.correlation_id.clone() {
            gateway_event = gateway_event.with_correlation_id(correlation_id);
        }
        self.event_bus.publish(gateway_event);

        let mut uplink_event = SystemEvent::new(
            EventKind::UplinkObservation,
            EventSource::Udp,
            "uplink.observed",
            EventStatus::Succeeded,
        )
        .with_entity_id(gateway_eui)
        .with_metadata(
            "signal_quality",
            format!("{:?}", command.uplink.signal_quality_hint()),
        )
        .with_metadata("payload_size", command.uplink.payload.len().to_string());
        if let Some(correlation_id) = command.correlation_id {
            uplink_event = uplink_event.with_correlation_id(correlation_id);
        }
        self.event_bus.publish(uplink_event);

        Ok(())
    }
}
