use async_trait::async_trait;
use maverick_domain::DevEui;

use crate::error::AppResult;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownlinkEnqueue {
    pub dev_eui: DevEui,
    pub f_port: u8,
    pub payload: Vec<u8>,
    pub confirmed: bool,
}

#[async_trait]
pub trait DownlinkRepository: Send + Sync {
    async fn enqueue(&self, item: &DownlinkEnqueue) -> AppResult<u64>;
}
