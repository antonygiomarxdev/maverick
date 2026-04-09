use serde::{Deserialize, Serialize};

use crate::types::{Eui64, Frequency, Rssi, Snr, SpreadingFactor};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UplinkFrame {
    pub gateway_eui: Eui64,
    pub dev_eui: Option<Eui64>,
    pub payload: Vec<u8>,
    pub f_port: Option<u8>,
    pub rssi: Rssi,
    pub snr: Snr,
    pub frequency: Frequency,
    pub spreading_factor: SpreadingFactor,
    pub timestamp: i64,
    pub frame_counter: Option<u32>,
    pub metadata: UplinkFrameMetadata,
    pub raw_frame: Vec<u8>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UplinkFrameMetadata {
    pub channel: u8,
    pub code_rate: Option<String>,
    pub crc_status: Option<u8>,
    pub modulation: Option<String>,
    pub bandwidth: Option<u32>,
    pub timestamp_ns: Option<u64>,
}

impl UplinkFrame {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        gateway_eui: Eui64,
        payload: Vec<u8>,
        rssi: Rssi,
        snr: Snr,
        frequency: Frequency,
        spreading_factor: SpreadingFactor,
        timestamp: i64,
        raw_frame: Vec<u8>,
    ) -> Self {
        Self {
            gateway_eui,
            dev_eui: None,
            payload,
            f_port: None,
            rssi,
            snr,
            frequency,
            spreading_factor,
            timestamp,
            frame_counter: None,
            metadata: UplinkFrameMetadata::default(),
            raw_frame,
        }
    }

    pub fn signal_quality_hint(&self) -> SignalQualityHint {
        let snr = self.snr.as_f32();

        if snr >= 5.0 {
            SignalQualityHint::Excellent
        } else if snr >= 0.0 {
            SignalQualityHint::Good
        } else if snr >= -5.0 {
            SignalQualityHint::Fair
        } else {
            SignalQualityHint::Poor
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignalQualityHint {
    Excellent,
    Good,
    Fair,
    Poor,
}
