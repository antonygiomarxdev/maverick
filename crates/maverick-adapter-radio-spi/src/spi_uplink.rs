//! ## SPI Adapter — UplinkObservation Parsing Contract
//!
//! When integrating libloragw RX (lgw_receive), the SPI adapter MUST:
//!
//! 1. Extract `wire_mic = phy_payload[phy_payload.len()-4..]` (last 4 bytes)
//! 2. Extract `phy_without_mic = &phy_payload[..phy_payload.len()-4]`
//! 3. Extract DevAddr, FCnt, FPort, payload per LoRaWAN 1.0.x PHY format
//! 4. Pass ALL of the above to UplinkObservation
//!
//! Without `wire_mic` and `phy_without_mic`, MIC verification in IngestUplink
//! will receive zeros and ALL valid frames will be rejected.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use maverick_core::error::{AppError, AppResult};
use maverick_core::ports::{UplinkReceive, UplinkSource};
use maverick_domain::{Eui64, GatewayEui};

use crate::lgw_bindings::lgw_pkt_rx_s;
use crate::lgw_convert::lgw_pkt_rx_to_observation;
use crate::lgw_init::{lgw_hal_start, lgw_hal_stop};

const GATEWAY_EUI: GatewayEui = GatewayEui(Eui64([0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]));

#[derive(Clone)]
pub struct SpiUplinkSource {
    inner: Arc<SpiInner>,
}

struct SpiInner {
    hal_lock: std::sync::Mutex<()>,
    idle_timeout: Duration,
}

impl SpiUplinkSource {
    pub fn new(spi_path: String, idle_timeout: Duration) -> AppResult<Self> {
        let trimmed = spi_path.trim();
        if trimmed.is_empty() {
            return Err(AppError::InvalidInput(
                "spi_path must not be empty for SpiUplinkSource".to_string(),
            ));
        }

        lgw_hal_start(trimmed)?;

        Ok(Self {
            inner: Arc::new(SpiInner {
                hal_lock: std::sync::Mutex::new(()),
                idle_timeout,
            }),
        })
    }

    fn blocking_receive(&self) -> AppResult<UplinkReceive> {
        let _guard = self
            .inner
            .hal_lock
            .lock()
            .map_err(|_| AppError::Infrastructure("spi hal mutex poisoned".to_string()))?;

        let mut pkt_data = [unsafe { std::mem::zeroed::<lgw_pkt_rx_s>() }; 16];
        let count = unsafe { crate::lgw_bindings::lgw_receive(16, pkt_data.as_mut_ptr()) };

        if count < 0 {
            return Err(AppError::Infrastructure(format!(
                "lgw_receive failed: {}",
                count
            )));
        }

        if count == 0 {
            std::thread::sleep(self.inner.idle_timeout);
            return Ok(UplinkReceive::Idle);
        }

        let mut observations = Vec::with_capacity(count as usize);
        for pkt in &pkt_data[..count as usize] {
            match lgw_pkt_rx_to_observation(pkt, GATEWAY_EUI) {
                Ok(obs) => observations.push(obs),
                Err(e) => {
                    tracing::warn!("failed to convert lgw_pkt_rx_s to UplinkObservation: {}", e);
                    continue;
                }
            }
        }

        Ok(UplinkReceive::Observations(observations))
    }
}

#[async_trait]
impl UplinkSource for SpiUplinkSource {
    async fn next_batch(&self) -> AppResult<UplinkReceive> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.blocking_receive())
            .await
            .map_err(|e| AppError::Infrastructure(format!("spi uplink join: {}", e)))?
    }
}

impl Drop for SpiUplinkSource {
    fn drop(&mut self) {
        lgw_hal_stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_mic_split_is_correct() {
        let payload: [u8; 23] = [
            0x40, 0x01, 0x02, 0x03, 0x04, 0x00, 0x00, 0x01, 0x01, 0x00, 0x11, 0x22, 0x33, 0x44,
            0x55, 0x66, 0x77, 0x88, 0xAA, 0xBB, 0xCC, 0xDD,
        ];
        let size = payload.len();

        let wire_mic: [u8; 4] = payload[size - 4..].try_into().unwrap();
        assert_eq!(wire_mic, [0xAA, 0xBB, 0xCC, 0xDD]);

        let phy_without_mic = &payload[..size - 4];
        assert_eq!(phy_without_mic.len(), 19);
        assert_eq!(phy_without_mic[phy_without_mic.len() - 1], 0x88);
    }
}
