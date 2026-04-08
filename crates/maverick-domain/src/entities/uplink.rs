use serde::{Deserialize, Serialize};
use crate::types::{Eui64, Frequency, Rssi, Snr, SpreadingFactor};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Uplink {
    pub payload: Vec<u8>,
    pub f_port: u8,
    pub dev_eui: Eui64,
    pub gateway_eui: Eui64,
    pub rssi: Rssi,
    pub snr: Snr,
    pub frequency: Frequency,
    pub spreading_factor: SpreadingFactor,
    pub timestamp: i64,
    pub frame_counter: u32,
    pub metadata: UplinkMetadata,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UplinkMetadata {
    pub channel: u8,
    pub code_rate: Option<String>,
    pub crc_status: Option<u8>,
    pub modulation: Option<String>,
    pub bandwidth: Option<u32>,
    pub timestamp_ns: Option<u64>,
}

impl Uplink {
    pub fn new(
        payload: Vec<u8>,
        f_port: u8,
        dev_eui: Eui64,
        gateway_eui: Eui64,
        rssi: Rssi,
        snr: Snr,
        frequency: Frequency,
        spreading_factor: SpreadingFactor,
        timestamp: i64,
        frame_counter: u32,
    ) -> Self {
        Self {
            payload,
            f_port,
            dev_eui,
            gateway_eui,
            rssi,
            snr,
            frequency,
            spreading_factor,
            timestamp,
            frame_counter,
            metadata: UplinkMetadata::default(),
        }
    }

    pub fn signal_quality(&self) -> SignalQuality {
        let snr = self.snr.as_f32();
        if snr >= 5.0 {
            SignalQuality::Excellent
        } else if snr >= 0.0 {
            SignalQuality::Good
        } else if snr >= -5.0 {
            SignalQuality::Fair
        } else {
            SignalQuality::Poor
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalQuality {
    Excellent,
    Good,
    Fair,
    Poor,
}