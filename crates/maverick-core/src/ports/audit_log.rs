use async_trait::async_trait;

use crate::error::Result;
use crate::events::AuditRecord;

#[async_trait]
pub trait AuditLogWriter: Send + Sync {
    async fn record(&self, record: AuditRecord) -> Result<()>;
}
