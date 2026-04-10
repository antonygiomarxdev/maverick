use crate::identifiers::{DevAddr, DevEui};
use crate::region::RegionId;

/// LoRaWAN device class (v1 ships Class A only; enum preserves evolution path).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DeviceClass {
    ClassA,
    ClassB,
    ClassC,
}

/// Protocol generation for capability-module routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(non_camel_case_types)]
pub enum LoRaWANVersion {
    V1_0_x,
}

/// Minimal session view the core needs for uplink validation (expand in later slices).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SessionSnapshot {
    pub dev_eui: DevEui,
    pub dev_addr: DevAddr,
    pub region: RegionId,
    pub class: DeviceClass,
    pub uplink_frame_counter: u32,
    pub downlink_frame_counter: u32,
}
