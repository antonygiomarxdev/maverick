use async_trait::async_trait;
use maverick_domain::DevEui;

use crate::error::AppResult;

/// Device registry port (persistence-agnostic).
#[async_trait]
pub trait DeviceRepository: Send + Sync {
    async fn exists(&self, dev_eui: DevEui) -> AppResult<bool>;
}
