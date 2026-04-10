use async_trait::async_trait;
use maverick_domain::DevAddr;

use crate::error::AppResult;

/// Persisted uplink record (minimal for v1 skeleton).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UplinkRecord {
    pub dev_addr: DevAddr,
    pub f_cnt: u32,
    pub payload: Vec<u8>,
}

#[async_trait]
pub trait UplinkRepository: Send + Sync {
    async fn append(&self, record: &UplinkRecord) -> AppResult<()>;
}
