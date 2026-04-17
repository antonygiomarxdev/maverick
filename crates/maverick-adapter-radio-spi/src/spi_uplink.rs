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

#[cfg(test)]
mod tests {
    /// SPI Adapter UplinkObservation Parsing Contract.
    ///
    /// When libloragw RX is integrated, this module verifies the contract is met:
    ///
    /// 1. Parse raw LoRaWAN PHY bytes from the concentrator
    /// 2. Construct UplinkObservation with fields:
    ///    - dev_addr: DevAddr — from FHDR bytes [1-4]
    ///    - f_cnt: u16 — from FHDR bytes [6-7] (wire value)
    ///    - f_port: u8 — after FHDR + FOpts
    ///    - payload: Vec<u8> — FRMPayload bytes (between FPort and MIC)
    ///    - wire_mic: [u8; 4] — last 4 bytes of PHY payload
    ///    - phy_without_mic: Vec<u8> — PHY payload excluding last 4 bytes
    ///    - gateway_eui: GatewayEui — from concentrator metadata
    ///    - region: RegionId — from frequency
    ///    - rssi: Option<i16>, snr: Option<f32> — from radio metadata
    ///
    /// Without wire_mic and phy_without_mic, MIC verification will receive
    /// zeros and ALL valid frames will be rejected.
    ///
    /// IMPLEMENTATION PENDING: This test will be implemented when libloragw
    /// integration begins.
    #[test]
    #[ignore = "pending libloragw integration"]
    fn spi_adapter_parsing_contract() {
        // Placeholder — implementation pending libloragw integration
        // Once real implementation is added, remove #[ignore] and implement test
    }
}
