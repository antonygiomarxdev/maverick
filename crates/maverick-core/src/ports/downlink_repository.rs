use async_trait::async_trait;
use maverick_domain::{Downlink, Eui64};

use crate::error::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownlinkState {
    Queued,
    Scheduled,
    Sent,
    Failed,
}

#[derive(Debug, Clone)]
pub struct QueuedDownlink {
    pub id: i64,
    pub downlink: Downlink,
    pub state: DownlinkState,
    pub attempt_count: u32,
    pub last_error: Option<String>,
    pub sent_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[async_trait]
pub trait DownlinkRepository: Send + Sync {
    async fn enqueue(&self, downlink: Downlink) -> Result<i64>;
    async fn get_by_id(&self, id: i64) -> Result<Option<QueuedDownlink>>;
    async fn list_by_dev_eui(
        &self,
        dev_eui: Eui64,
        state: Option<DownlinkState>,
        limit: usize,
    ) -> Result<Vec<QueuedDownlink>>;
    async fn list_pending(&self, limit: usize) -> Result<Vec<QueuedDownlink>>;
    async fn mark_scheduled(&self, id: i64, scheduled_at: i64) -> Result<()>;
    async fn mark_sent(&self, id: i64, sent_at: i64) -> Result<()>;
    async fn mark_retry(&self, id: i64, retry_at: i64, reason: &str) -> Result<()>;
    async fn mark_retry_with_gateway(
        &self,
        id: i64,
        retry_at: i64,
        gateway_eui: Eui64,
        reason: &str,
    ) -> Result<()>;
    async fn mark_failed(&self, id: i64, reason: &str) -> Result<()>;
}

#[async_trait]
impl<T> DownlinkRepository for &T
where
    T: DownlinkRepository + Sync,
{
    async fn enqueue(&self, downlink: Downlink) -> Result<i64> {
        (**self).enqueue(downlink).await
    }

    async fn get_by_id(&self, id: i64) -> Result<Option<QueuedDownlink>> {
        (**self).get_by_id(id).await
    }

    async fn list_by_dev_eui(
        &self,
        dev_eui: Eui64,
        state: Option<DownlinkState>,
        limit: usize,
    ) -> Result<Vec<QueuedDownlink>> {
        (**self).list_by_dev_eui(dev_eui, state, limit).await
    }

    async fn list_pending(&self, limit: usize) -> Result<Vec<QueuedDownlink>> {
        (**self).list_pending(limit).await
    }

    async fn mark_scheduled(&self, id: i64, scheduled_at: i64) -> Result<()> {
        (**self).mark_scheduled(id, scheduled_at).await
    }

    async fn mark_sent(&self, id: i64, sent_at: i64) -> Result<()> {
        (**self).mark_sent(id, sent_at).await
    }

    async fn mark_retry(&self, id: i64, retry_at: i64, reason: &str) -> Result<()> {
        (**self).mark_retry(id, retry_at, reason).await
    }

    async fn mark_retry_with_gateway(
        &self,
        id: i64,
        retry_at: i64,
        gateway_eui: Eui64,
        reason: &str,
    ) -> Result<()> {
        (**self)
            .mark_retry_with_gateway(id, retry_at, gateway_eui, reason)
            .await
    }

    async fn mark_failed(&self, id: i64, reason: &str) -> Result<()> {
        (**self).mark_failed(id, reason).await
    }
}
