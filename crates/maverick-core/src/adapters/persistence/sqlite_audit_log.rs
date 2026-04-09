use std::sync::Arc;

use async_trait::async_trait;

use crate::adapters::persistence::sqlite_utils::{optional_text_literal, text_literal};
use crate::db::Database;
use crate::events::AuditRecord;
use crate::ports::AuditLogWriter;
use crate::Result;

pub struct SqliteAuditLogWriter<D: Database> {
    db: Arc<D>,
}

impl<D: Database> SqliteAuditLogWriter<D> {
    pub fn new(db: Arc<D>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl<D: Database> AuditLogWriter for SqliteAuditLogWriter<D> {
    async fn record(&self, record: AuditRecord) -> Result<()> {
        let metadata_json = serde_json::to_string(&record.metadata)
            .map_err(|err| crate::AppError::Event(err.to_string()))?;

        let query = format!(
            "INSERT INTO audit_log (source, operation, entity_type, entity_id, outcome, reason_code, correlation_id, summary, metadata_json, created_at) VALUES ({}, {}, {}, {}, {}, {}, {}, {}, {}, {})",
            text_literal(event_source_name(&record.source)),
            text_literal(&record.operation),
            text_literal(&record.entity_type),
            optional_text_literal(record.entity_id.as_deref()),
            text_literal(event_status_name(&record.outcome)),
            optional_text_literal(record.reason_code.as_deref()),
            optional_text_literal(record.correlation_id.as_deref()),
            text_literal(&record.summary),
            text_literal(&metadata_json),
            record.timestamp,
        );

        self.db.execute(&query).await?;
        Ok(())
    }
}

fn event_source_name(source: &crate::events::EventSource) -> &'static str {
    match source {
        crate::events::EventSource::Api => "Api",
        crate::events::EventSource::Udp => "Udp",
        crate::events::EventSource::Database => "Database",
        crate::events::EventSource::System => "System",
    }
}

fn event_status_name(status: &crate::events::EventStatus) -> &'static str {
    match status {
        crate::events::EventStatus::Accepted => "Accepted",
        crate::events::EventStatus::Succeeded => "Succeeded",
        crate::events::EventStatus::Rejected => "Rejected",
        crate::events::EventStatus::Failed => "Failed",
    }
}
