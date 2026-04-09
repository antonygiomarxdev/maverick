use async_trait::async_trait;
use maverick_domain::{Eui64, Gateway};

use crate::error::Result;

#[async_trait]
pub trait GatewayRepository: Send + Sync {
    async fn create(&self, gateway: Gateway) -> Result<Gateway>;
    async fn update(&self, gateway: Gateway) -> Result<Gateway>;
    async fn delete(&self, gateway_eui: Eui64) -> Result<()>;
    async fn get_by_gateway_eui(&self, gateway_eui: Eui64) -> Result<Option<Gateway>>;
}
