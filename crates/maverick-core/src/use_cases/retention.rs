use std::sync::Arc;
use std::time::Duration;

use crate::db::Database;
use crate::events::{EventBus, EventKind, EventSource, EventStatus, SystemEvent};
use crate::storage_profile::StorageLimits;

const RETENTION_SWEEP_INTERVAL_SECS: u64 = 3600;

pub struct RetentionService<D: Database> {
    db: Arc<D>,
    limits: StorageLimits,
    event_bus: EventBus,
}

impl<D: Database + 'static> RetentionService<D> {
    pub fn new(db: Arc<D>, limits: StorageLimits, event_bus: EventBus) -> Self {
        Self {
            db,
            limits,
            event_bus,
        }
    }

    pub async fn run_forever(self) {
        tracing::info!(
            retention_days = self.limits.retention_days,
            "retention service started"
        );
        loop {
            tokio::time::sleep(Duration::from_secs(RETENTION_SWEEP_INTERVAL_SECS)).await;
            if let Err(e) = self.sweep().await {
                tracing::warn!(error = %e, "retention sweep failed");
            }
        }
    }

    async fn sweep(&self) -> crate::Result<()> {
        let uplink_rows = self
            .db
            .execute(
                "DELETE FROM uplinks WHERE expires_at IS NOT NULL AND expires_at < unixepoch()",
            )
            .await?
            .affected_rows;

        let audit_rows = self
            .db
            .execute(
                "DELETE FROM audit_log WHERE expires_at IS NOT NULL AND expires_at < unixepoch()",
            )
            .await?
            .affected_rows;

        if uplink_rows > 0 || audit_rows > 0 {
            tracing::info!(
                uplinks_purged = uplink_rows,
                audit_log_purged = audit_rows,
                retention_days = self.limits.retention_days,
                "retention sweep completed"
            );
            self.event_bus.publish(
                SystemEvent::new(
                    EventKind::RetentionPurge,
                    EventSource::Udp,
                    "storage.retention.purged",
                    EventStatus::Succeeded,
                )
                .with_metadata("uplinks_purged", uplink_rows.to_string())
                .with_metadata("audit_log_purged", audit_rows.to_string())
                .with_metadata("retention_days", self.limits.retention_days.to_string()),
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::SqliteDb;
    use crate::db::Value;
    use crate::events::EventBus;
    use crate::storage_profile::{StorageLimits, StorageProfile};
    use std::sync::Arc;

    #[tokio::test]
    async fn sweep_purges_expired_uplinks() {
        let db = SqliteDb::in_memory().await.expect("in-memory db");
        let db_arc = Arc::new(db);

        // Insert an expired row and a non-expired row directly
        db_arc
                .execute(
                    "INSERT INTO uplinks (gateway_eui, payload, rssi, snr, frequency_hz, spreading_factor, received_at, expires_at) \
                     VALUES (X'AABBCCDDEEFF0001', X'01', -80, 7.0, 868100000, 7, 0, 0)",
                )
                .await
                .expect("insert expired uplink");
        db_arc
                .execute(
                    "INSERT INTO uplinks (gateway_eui, payload, rssi, snr, frequency_hz, spreading_factor, received_at, expires_at) \
                     VALUES (X'AABBCCDDEEFF0002', X'02', -80, 7.0, 868100000, 7, 0, 9999999999)",
                )
                .await
                .expect("insert non-expired uplink");

        let limits = StorageLimits::for_profile(StorageProfile::Mid);
        let event_bus = EventBus::new(16);
        let svc = RetentionService::new(db_arc.clone(), limits, event_bus);
        svc.sweep().await.expect("sweep");

        let rows = db_arc
            .query("SELECT COUNT(*) as cnt FROM uplinks")
            .await
            .expect("count");
        let count = match rows.first().and_then(|r| r.values.first()) {
            Some(Value::Integer(n)) => *n,
            _ => 0,
        };
        assert_eq!(count, 1, "only the non-expired row should remain");
    }
}
