use maverick_domain::{DeviceClass, LoRaWANVersion, RegionId, SessionSnapshot};

use crate::error::AppResult;
use crate::ports::UplinkObservation;

/// Context passed to a protocol capability module for a single uplink.
#[derive(Debug, Clone)]
pub struct ProtocolContext<'a> {
    pub observation: &'a UplinkObservation,
    pub session: Option<&'a SessionSnapshot>,
}

/// Result of protocol policy evaluation (expand with MAC commands later).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolDecision {
    Accept,
    RejectDuplicateFrameCounter,
    RejectFcntGapExceeded,
    RejectNoSession,
    RejectRegionMismatch,
    RejectUnsupportedClass,
}

/// Pluggable LoRaWAN behavior (version/class). v1 ships `LoRaWAN10xClassA` only.
pub trait ProtocolCapability: Send + Sync {
    fn id(&self) -> &'static str;

    fn supports(&self, version: LoRaWANVersion, class: DeviceClass, region: RegionId) -> bool;

    fn validate_uplink(&self, ctx: ProtocolContext<'_>) -> AppResult<ProtocolDecision>;
}
