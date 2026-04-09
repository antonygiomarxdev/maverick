use async_trait::async_trait;
use maverick_domain::{Eui64, Gateway};

use crate::error::Result;

#[async_trait]
pub trait GatewayRepository: Send + Sync {
    async fn upsert(&self, gateway: Gateway) -> Result<Gateway>;
    async fn get_by_gateway_eui(&self, gateway_eui: Eui64) -> Result<Option<Gateway>>;
}
