use std::sync::Arc;

use async_trait::async_trait;
use maverick_domain::UplinkFrame;

use crate::error::Result;

#[async_trait]
pub trait UplinkRepository: Send + Sync {
    async fn append(&self, uplink: UplinkFrame) -> Result<()>;
    async fn append_batch(&self, uplinks: Vec<UplinkFrame>) -> Result<()>;
}

#[async_trait]
impl UplinkRepository for Arc<dyn UplinkRepository + Send + Sync> {
    async fn append(&self, uplink: UplinkFrame) -> Result<()> {
        (**self).append(uplink).await
    }

    async fn append_batch(&self, uplinks: Vec<UplinkFrame>) -> Result<()> {
        (**self).append_batch(uplinks).await
    }
}
