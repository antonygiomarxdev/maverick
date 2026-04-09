use async_trait::async_trait;
use maverick_domain::{DeviceSession, Eui64};

use crate::error::Result;

#[async_trait]
pub trait SessionRepository: Send + Sync {
    async fn upsert_for_device(&self, dev_eui: Eui64, session: DeviceSession) -> Result<()>;
    async fn get_by_dev_eui(&self, dev_eui: Eui64) -> Result<Option<DeviceSession>>;
    async fn get_by_dev_addr(&self, dev_addr: u32) -> Result<Option<(Eui64, DeviceSession)>>;
}

#[async_trait]
impl<T> SessionRepository for &T
where
    T: SessionRepository + Sync,
{
    async fn upsert_for_device(&self, dev_eui: Eui64, session: DeviceSession) -> Result<()> {
        (**self).upsert_for_device(dev_eui, session).await
    }

    async fn get_by_dev_eui(&self, dev_eui: Eui64) -> Result<Option<DeviceSession>> {
        (**self).get_by_dev_eui(dev_eui).await
    }

    async fn get_by_dev_addr(&self, dev_addr: u32) -> Result<Option<(Eui64, DeviceSession)>> {
        (**self).get_by_dev_addr(dev_addr).await
    }
}
