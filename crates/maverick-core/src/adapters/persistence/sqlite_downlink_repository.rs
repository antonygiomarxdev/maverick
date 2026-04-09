use std::sync::Arc;

use async_trait::async_trait;
use maverick_domain::{Downlink, DownlinkPriority, Eui64, Frequency, SpreadingFactor};

use crate::adapters::persistence::sqlite_utils::{
    blob_literal, optional_i64, optional_text, optional_text_literal, required_blob, required_i64,
    required_text,
};
use crate::db::{Database, Row};
use crate::error::{AppError, Result};
use crate::ports::{DownlinkRepository, DownlinkState, QueuedDownlink};

pub struct SqliteDownlinkRepository<D: Database> {
    db: Arc<D>,
}

impl<D: Database> SqliteDownlinkRepository<D> {
    pub fn new(db: Arc<D>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl<D: Database> DownlinkRepository for SqliteDownlinkRepository<D> {
    async fn enqueue(&self, downlink: Downlink) -> Result<i64> {
        let query = format!(
            "INSERT INTO downlinks (dev_eui, gateway_eui, payload, f_port, frequency_hz, spreading_factor, frame_counter, priority, scheduled_at, state, attempt_count, last_error, sent_at, created_at, updated_at) VALUES ({}, {}, {}, {}, {}, {}, {}, {}, {}, 'Queued', 0, NULL, NULL, unixepoch(), unixepoch())",
            blob_literal(downlink.dev_eui.as_bytes_slice()),
            blob_literal(downlink.gateway_eui.as_bytes_slice()),
            blob_literal(&downlink.payload),
            downlink.f_port,
            downlink.frequency.as_hz(),
            downlink.spreading_factor.0,
            downlink.frame_counter,
            priority_literal(downlink.priority),
            optional_i64(downlink.scheduled_at),
        );

        let result = self.db.execute(&query).await?;
        result.last_insert_id.ok_or_else(|| {
            AppError::Database("downlink enqueue did not return insert id".to_string())
        })
    }

    async fn get_by_id(&self, id: i64) -> Result<Option<QueuedDownlink>> {
        let query = format!(
            "SELECT id, dev_eui, gateway_eui, payload, f_port, frequency_hz, spreading_factor, frame_counter, priority, scheduled_at, state, attempt_count, last_error, sent_at, created_at, updated_at FROM downlinks WHERE id = {} LIMIT 1",
            id
        );
        let rows = self.db.query(&query).await?;

        rows.into_iter().next().map(map_downlink_row).transpose()
    }

    async fn list_pending(&self, limit: usize) -> Result<Vec<QueuedDownlink>> {
        let query = format!(
            "SELECT id, dev_eui, gateway_eui, payload, f_port, frequency_hz, spreading_factor, frame_counter, priority, scheduled_at, state, attempt_count, last_error, sent_at, created_at, updated_at FROM downlinks WHERE state IN ('Queued', 'Scheduled') ORDER BY CASE priority WHEN 'Critical' THEN 3 WHEN 'High' THEN 2 WHEN 'Normal' THEN 1 ELSE 0 END DESC, created_at ASC LIMIT {}",
            limit
        );
        let rows = self.db.query(&query).await?;
        rows.into_iter().map(map_downlink_row).collect()
    }

    async fn mark_scheduled(&self, id: i64, scheduled_at: i64) -> Result<()> {
        let query = format!(
            "UPDATE downlinks SET state = 'Scheduled', scheduled_at = {}, updated_at = unixepoch() WHERE id = {}",
            scheduled_at, id
        );
        self.db.execute(&query).await?;
        Ok(())
    }

    async fn mark_sent(&self, id: i64, sent_at: i64) -> Result<()> {
        let query = format!(
            "UPDATE downlinks SET state = 'Sent', sent_at = {}, updated_at = unixepoch() WHERE id = {}",
            sent_at, id
        );
        self.db.execute(&query).await?;
        Ok(())
    }

    async fn mark_retry(&self, id: i64, retry_at: i64, reason: &str) -> Result<()> {
        let query = format!(
            "UPDATE downlinks SET state = 'Queued', attempt_count = attempt_count + 1, last_error = {}, scheduled_at = {}, updated_at = unixepoch() WHERE id = {}",
            optional_text_literal(Some(reason)),
            retry_at,
            id
        );
        self.db.execute(&query).await?;
        Ok(())
    }

    async fn mark_failed(&self, id: i64, reason: &str) -> Result<()> {
        let query = format!(
            "UPDATE downlinks SET state = 'Failed', attempt_count = attempt_count + 1, last_error = {}, updated_at = unixepoch() WHERE id = {}",
            optional_text_literal(Some(reason)),
            id
        );
        self.db.execute(&query).await?;
        Ok(())
    }
}

fn map_downlink_row(row: Row) -> Result<QueuedDownlink> {
    let id = required_i64(&row, 0, "id")?;
    let dev_eui = Eui64::from(required_blob::<8>(&row, 1, "dev_eui")?);
    let gateway_eui = Eui64::from(required_blob::<8>(&row, 2, "gateway_eui")?);
    let payload = match row.values.get(3) {
        Some(crate::db::Value::Blob(bytes)) => bytes.clone(),
        _ => {
            return Err(AppError::Database(
                "invalid payload for downlink".to_string(),
            ))
        }
    };
    let f_port = required_i64(&row, 4, "f_port")? as u8;
    let frequency = Frequency::new(required_i64(&row, 5, "frequency_hz")? as u32);
    let spreading_factor_value = required_i64(&row, 6, "spreading_factor")? as u8;
    let spreading_factor = SpreadingFactor::new(spreading_factor_value).ok_or_else(|| {
        AppError::Database(format!(
            "invalid spreading_factor '{}' for downlink row",
            spreading_factor_value
        ))
    })?;
    let frame_counter = required_i64(&row, 7, "frame_counter")? as u32;
    let priority = parse_priority(&required_text(&row, 8, "priority")?)?;
    let scheduled_at = optional_i64_value(&row, 9);
    let state = parse_state(&required_text(&row, 10, "state")?)?;
    let attempt_count = required_i64(&row, 11, "attempt_count")? as u32;
    let last_error = optional_text(&row, 12);
    let sent_at = optional_i64_value(&row, 13);
    let created_at = required_i64(&row, 14, "created_at")?;
    let updated_at = required_i64(&row, 15, "updated_at")?;

    let mut downlink = Downlink::new(
        payload,
        f_port,
        dev_eui,
        gateway_eui,
        frequency,
        spreading_factor,
        created_at,
        frame_counter,
    )
    .with_priority(priority);
    downlink.scheduled_at = scheduled_at;

    Ok(QueuedDownlink {
        id,
        downlink,
        state,
        attempt_count,
        last_error,
        sent_at,
        created_at,
        updated_at,
    })
}

fn priority_literal(priority: DownlinkPriority) -> String {
    match priority {
        DownlinkPriority::Low => "'Low'",
        DownlinkPriority::Normal => "'Normal'",
        DownlinkPriority::High => "'High'",
        DownlinkPriority::Critical => "'Critical'",
    }
    .to_string()
}

fn parse_priority(value: &str) -> Result<DownlinkPriority> {
    match value {
        "Low" => Ok(DownlinkPriority::Low),
        "Normal" => Ok(DownlinkPriority::Normal),
        "High" => Ok(DownlinkPriority::High),
        "Critical" => Ok(DownlinkPriority::Critical),
        _ => Err(AppError::Database(format!(
            "invalid downlink priority '{}'",
            value
        ))),
    }
}

fn parse_state(value: &str) -> Result<DownlinkState> {
    match value {
        "Queued" => Ok(DownlinkState::Queued),
        "Scheduled" => Ok(DownlinkState::Scheduled),
        "Sent" => Ok(DownlinkState::Sent),
        "Failed" => Ok(DownlinkState::Failed),
        _ => Err(AppError::Database(format!(
            "invalid downlink state '{}'",
            value
        ))),
    }
}

fn optional_i64_value(row: &Row, index: usize) -> Option<i64> {
    match row.values.get(index) {
        Some(crate::db::Value::Integer(value)) => Some(*value),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use maverick_domain::{Downlink, DownlinkPriority, Eui64, Frequency, SpreadingFactor};

    use crate::adapters::persistence::SqliteDownlinkRepository;
    use crate::db::SqliteDb;
    use crate::ports::{DownlinkRepository, DownlinkState};

    fn sample_downlink() -> Downlink {
        Downlink::new(
            vec![0x01, 0x02],
            10,
            Eui64::from([1, 2, 3, 4, 5, 6, 7, 8]),
            Eui64::from([8, 7, 6, 5, 4, 3, 2, 1]),
            Frequency::new(868_100_000),
            SpreadingFactor::new(7).expect("spreading factor must be valid"),
            0,
            42,
        )
        .with_priority(DownlinkPriority::High)
    }

    #[tokio::test]
    async fn enqueue_and_transition_states() {
        let db = Arc::new(SqliteDb::in_memory().await.expect("db must open"));
        let repository = SqliteDownlinkRepository::new(db);
        let id = repository
            .enqueue(sample_downlink())
            .await
            .expect("enqueue must succeed");

        let queued = repository
            .get_by_id(id)
            .await
            .expect("query must succeed")
            .expect("downlink must exist");
        assert_eq!(queued.state, DownlinkState::Queued);

        repository
            .mark_scheduled(id, 123)
            .await
            .expect("mark scheduled must succeed");
        let scheduled = repository
            .get_by_id(id)
            .await
            .expect("query must succeed")
            .expect("downlink must exist");
        assert_eq!(scheduled.state, DownlinkState::Scheduled);

        repository
            .mark_sent(id, 124)
            .await
            .expect("mark sent must succeed");
        let sent = repository
            .get_by_id(id)
            .await
            .expect("query must succeed")
            .expect("downlink must exist");
        assert_eq!(sent.state, DownlinkState::Sent);
        assert_eq!(sent.sent_at, Some(124));

        repository
            .mark_retry(id, 130, "temporary gateway timeout")
            .await
            .expect("mark retry must succeed");
        let retried = repository
            .get_by_id(id)
            .await
            .expect("query must succeed")
            .expect("downlink must exist");
        assert_eq!(retried.state, DownlinkState::Queued);
        assert_eq!(retried.attempt_count, 1);
        assert_eq!(
            retried.last_error.as_deref(),
            Some("temporary gateway timeout")
        );
        assert_eq!(retried.downlink.scheduled_at, Some(130));
    }
}
