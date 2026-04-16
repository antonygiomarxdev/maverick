//! `UplinkSource` over SPI — placeholder until libloragw RX is integrated.

use std::time::Duration;

use async_trait::async_trait;
use maverick_core::error::{AppError, AppResult};
use maverick_core::ports::{UplinkReceive, UplinkSource};

/// Blocking-style SPI / concentrator poll (libloragw hook point).
#[derive(Debug)]
pub struct SpiUplinkSource {
    spi_path: String,
    read_timeout: Duration,
}

impl SpiUplinkSource {
    pub fn new(spi_path: String, read_timeout: Duration) -> AppResult<Self> {
        let trimmed = spi_path.trim();
        if trimmed.is_empty() {
            return Err(AppError::InvalidInput(
                "spi_path must not be empty for SpiUplinkSource".to_string(),
            ));
        }
        Ok(Self {
            spi_path: trimmed.to_string(),
            read_timeout,
        })
    }

    fn blocking_poll(path: &str, idle_wait: Duration) -> AppResult<UplinkReceive> {
        std::fs::metadata(path).map_err(|e| {
            AppError::Infrastructure(format!("SPI device path not accessible ({path}): {e}"))
        })?;
        // Placeholder: real implementation will call libloragw `lgw_receive` (or equivalent).
        std::thread::sleep(idle_wait);
        Ok(UplinkReceive::Idle)
    }
}

#[async_trait]
impl UplinkSource for SpiUplinkSource {
    async fn next_batch(&self) -> AppResult<UplinkReceive> {
        let path = self.spi_path.clone();
        let idle = self.read_timeout;
        tokio::task::spawn_blocking(move || Self::blocking_poll(&path, idle))
            .await
            .map_err(|e| AppError::Infrastructure(format!("spi uplink join: {e}")))?
    }
}
