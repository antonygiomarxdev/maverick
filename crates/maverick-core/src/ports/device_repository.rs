use async_trait::async_trait;
use maverick_domain::{Device, Eui64};

use crate::error::Result;

#[async_trait]
pub trait DeviceRepository: Send + Sync {
    async fn create(&self, device: Device) -> Result<Device>;
    async fn get_by_dev_eui(&self, dev_eui: Eui64) -> Result<Option<Device>>;
    async fn update(&self, device: Device) -> Result<Device>;
    async fn delete(&self, dev_eui: Eui64) -> Result<()>;
}
