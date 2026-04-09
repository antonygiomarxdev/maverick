use crate::types::{Eui64, Frequency, SpreadingFactor};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Downlink {
    pub payload: Vec<u8>,
    pub f_port: u8,
    pub dev_eui: Eui64,
    pub gateway_eui: Eui64,
    pub frequency: Frequency,
    pub spreading_factor: SpreadingFactor,
    pub timestamp: i64,
    pub frame_counter: u32,
    pub priority: DownlinkPriority,
    pub scheduled_at: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DownlinkPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

impl Downlink {
    pub fn new(
        payload: Vec<u8>,
        f_port: u8,
        dev_eui: Eui64,
        gateway_eui: Eui64,
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
            frequency,
            spreading_factor,
            timestamp,
            frame_counter,
            priority: DownlinkPriority::Normal,
            scheduled_at: None,
        }
    }

    pub fn with_priority(mut self, priority: DownlinkPriority) -> Self {
        self.priority = priority;
        self
    }

    pub fn schedule(&mut self, scheduled_at: i64) {
        self.scheduled_at = Some(scheduled_at);
    }

    pub fn is_scheduled(&self) -> bool {
        self.scheduled_at.is_some()
    }
}
