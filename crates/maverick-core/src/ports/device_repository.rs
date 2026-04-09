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

#[async_trait]
impl<T> DeviceRepository for &T
where
    T: DeviceRepository + Sync,
{
    async fn create(&self, device: Device) -> Result<Device> {
        (**self).create(device).await
    }

    async fn get_by_dev_eui(&self, dev_eui: Eui64) -> Result<Option<Device>> {
        (**self).get_by_dev_eui(dev_eui).await
    }

    async fn update(&self, device: Device) -> Result<Device> {
        (**self).update(device).await
    }

    async fn delete(&self, dev_eui: Eui64) -> Result<()> {
        (**self).delete(dev_eui).await
    }
}
