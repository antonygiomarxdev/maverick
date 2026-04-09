use async_trait::async_trait;
use maverick_domain::{Eui64, Gateway, GatewayStatus};

use crate::error::Result;

#[async_trait]
pub trait GatewayRepository: Send + Sync {
    async fn create(&self, gateway: Gateway) -> Result<Gateway>;
    async fn update(&self, gateway: Gateway) -> Result<Gateway>;
    async fn delete(&self, gateway_eui: Eui64) -> Result<()>;
    async fn get_by_gateway_eui(&self, gateway_eui: Eui64) -> Result<Option<Gateway>>;
    async fn list(&self, status: Option<GatewayStatus>) -> Result<Vec<Gateway>>;
    async fn list_healthy(&self) -> Result<Vec<Gateway>>;
}

#[async_trait]
impl<T> GatewayRepository for &T
where
    T: GatewayRepository + Sync,
{
    async fn create(&self, gateway: Gateway) -> Result<Gateway> {
        (**self).create(gateway).await
    }

    async fn update(&self, gateway: Gateway) -> Result<Gateway> {
        (**self).update(gateway).await
    }

    async fn delete(&self, gateway_eui: Eui64) -> Result<()> {
        (**self).delete(gateway_eui).await
    }

    async fn get_by_gateway_eui(&self, gateway_eui: Eui64) -> Result<Option<Gateway>> {
        (**self).get_by_gateway_eui(gateway_eui).await
    }

    async fn list(&self, status: Option<GatewayStatus>) -> Result<Vec<Gateway>> {
        (**self).list(status).await
    }

    async fn list_healthy(&self) -> Result<Vec<Gateway>> {
        (**self).list_healthy().await
    }
}
