use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

#[derive(Debug, Clone)]
pub struct EventBus {
    sender: broadcast::Sender<SystemEvent>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    pub fn publish(&self, event: SystemEvent) {
        let _ = self.sender.send(event);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<SystemEvent> {
        self.sender.subscribe()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EventSource {
    Api,
    Udp,
    Database,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EventStatus {
    Accepted,
    Succeeded,
    Rejected,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EventKind {
    SystemLifecycle,
    DeviceCommand,
    DownlinkCommand,
    GatewayObservation,
    UplinkObservation,
    AuditRecord,
    StoragePressure,
    RetentionPurge,
    CircularDrop,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SystemEvent {
    pub kind: EventKind,
    pub source: EventSource,
    pub operation: String,
    pub status: EventStatus,
    pub entity_id: Option<String>,
    pub reason_code: Option<String>,
    pub correlation_id: Option<String>,
    pub timestamp: i64,
    pub metadata: BTreeMap<String, String>,
}

impl SystemEvent {
    pub fn new(
        kind: EventKind,
        source: EventSource,
        operation: impl Into<String>,
        status: EventStatus,
    ) -> Self {
        Self {
            kind,
            source,
            operation: operation.into(),
            status,
            entity_id: None,
            reason_code: None,
            correlation_id: None,
            timestamp: unix_timestamp(),
            metadata: BTreeMap::new(),
        }
    }

    pub fn with_entity_id(mut self, entity_id: impl Into<String>) -> Self {
        self.entity_id = Some(entity_id.into());
        self
    }

    pub fn with_reason_code(mut self, reason_code: impl Into<String>) -> Self {
        self.reason_code = Some(reason_code.into());
        self
    }

    pub fn with_correlation_id(mut self, correlation_id: impl Into<String>) -> Self {
        self.correlation_id = Some(correlation_id.into());
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuditRecord {
    pub source: EventSource,
    pub operation: String,
    pub entity_type: String,
    pub entity_id: Option<String>,
    pub outcome: EventStatus,
    pub reason_code: Option<String>,
    pub correlation_id: Option<String>,
    pub timestamp: i64,
    pub summary: String,
    pub metadata: BTreeMap<String, String>,
}

impl AuditRecord {
    pub fn new(
        source: EventSource,
        operation: impl Into<String>,
        entity_type: impl Into<String>,
        outcome: EventStatus,
        summary: impl Into<String>,
    ) -> Self {
        Self {
            source,
            operation: operation.into(),
            entity_type: entity_type.into(),
            entity_id: None,
            outcome,
            reason_code: None,
            correlation_id: None,
            timestamp: unix_timestamp(),
            summary: summary.into(),
            metadata: BTreeMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

fn unix_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{EventBus, EventKind, EventSource, EventStatus, SystemEvent};

    #[tokio::test]
    async fn event_bus_delivers_published_events() {
        let bus = EventBus::new(8);
        let mut subscriber = bus.subscribe();
        let event = SystemEvent::new(
            EventKind::SystemLifecycle,
            EventSource::System,
            "kernel.started",
            EventStatus::Succeeded,
        );

        bus.publish(event.clone());

        let received = subscriber.recv().await.expect("event must be received");
        assert_eq!(received, event);
    }
}
