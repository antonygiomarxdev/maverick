use async_trait::async_trait;
use maverick_domain::{DevAddr, DevEui};

use crate::error::AppResult;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownlinkEnqueue {
    pub dev_eui: DevEui,
    pub f_port: u8,
    pub payload: Vec<u8>,
    pub confirmed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownlinkItem {
    pub id: u64,
    pub dev_eui: DevEui,
    pub dev_addr: DevAddr,
    pub f_port: u8,
    pub payload: Vec<u8>,
    pub confirmed: bool,
    pub ack_flag: bool,
    pub enqueued_at_ms: i64,
    pub frame_counter: u32,
}

#[async_trait]
pub trait DownlinkRepository: Send + Sync {
    async fn enqueue(&self, item: &DownlinkEnqueue) -> AppResult<u64>;

    async fn dequeue_oldest(&self, dev_eui: &DevEui, limit: usize) -> AppResult<Vec<DownlinkItem>>;

    async fn mark_transmitted(&self, id: u64) -> AppResult<()>;

    async fn mark_failed(&self, id: u64) -> AppResult<()>;

    async fn get_pending_for_dev(&self, dev_eui: &DevEui) -> AppResult<Vec<DownlinkItem>>;
}
