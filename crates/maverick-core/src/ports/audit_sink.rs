use async_trait::async_trait;
use serde_json::Value;

use crate::error::AppResult;

#[derive(Debug, Clone)]
pub struct AuditRecord {
    pub source: String,
    pub operation: String,
    pub entity_type: String,
    pub entity_id: Option<String>,
    pub outcome: String,
    pub metadata: Option<Value>,
}

#[async_trait]
pub trait AuditSink: Send + Sync {
    async fn emit(&self, record: AuditRecord) -> AppResult<()>;
}
