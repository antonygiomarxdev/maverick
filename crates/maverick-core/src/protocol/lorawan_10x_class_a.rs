use maverick_domain::{DeviceClass, LoRaWANVersion, RegionId};

use crate::error::AppResult;
use crate::protocol::{ProtocolCapability, ProtocolContext, ProtocolDecision};

/// LoRaWAN 1.0.x Class A policy module (v1 baseline regions).
pub struct LoRaWAN10xClassA;

impl LoRaWAN10xClassA {
    fn region_supported(region: RegionId) -> bool {
        matches!(
            region,
            RegionId::Eu868 | RegionId::Us915 | RegionId::Au915 | RegionId::As923 | RegionId::Eu433
        )
    }
}

impl ProtocolCapability for LoRaWAN10xClassA {
    fn id(&self) -> &'static str {
        "lorawan_1_0_x_class_a"
    }

    fn supports(&self, version: LoRaWANVersion, class: DeviceClass, region: RegionId) -> bool {
        version == LoRaWANVersion::V1_0_x
            && class == DeviceClass::ClassA
            && Self::region_supported(region)
    }

    fn validate_uplink(&self, ctx: ProtocolContext<'_>) -> AppResult<ProtocolDecision> {
        let obs = ctx.observation;
        if !Self::region_supported(obs.region) {
            return Ok(ProtocolDecision::RejectRegionMismatch);
        }
        let Some(session) = ctx.session else {
            return Ok(ProtocolDecision::RejectNoSession);
        };
        if session.class != DeviceClass::ClassA {
            return Ok(ProtocolDecision::RejectUnsupportedClass);
        }
        if session.region != obs.region {
            return Ok(ProtocolDecision::RejectRegionMismatch);
        }
        // LoRaWAN 1.0.x: uplink FCnt must be strictly greater than last seen (32-bit).
        if obs.f_cnt <= session.uplink_frame_counter {
            return Ok(ProtocolDecision::RejectDuplicateFrameCounter);
        }
        Ok(ProtocolDecision::Accept)
    }
}

#[cfg(test)]
mod tests {
    use maverick_domain::identifiers::Eui64;
    use maverick_domain::{DevAddr, DevEui, SessionSnapshot};

    use super::*;
    use crate::ports::UplinkObservation;

    fn sample_session(fc: u32) -> SessionSnapshot {
        SessionSnapshot {
            dev_eui: DevEui(Eui64([1, 2, 3, 4, 5, 6, 7, 8])),
            dev_addr: DevAddr(0x01_02_03_04),
            region: RegionId::Eu868,
            class: DeviceClass::ClassA,
            uplink_frame_counter: fc,
            downlink_frame_counter: 0,
        }
    }

    fn sample_observation(fc: u32) -> UplinkObservation {
        UplinkObservation {
            gateway_eui: maverick_domain::GatewayEui(Eui64([8; 8])),
            dev_addr: DevAddr(0x01_02_03_04),
            region: RegionId::Eu868,
            f_cnt: fc,
            f_port: 1,
            payload: vec![0x01],
            rssi: None,
            snr: None,
        }
    }

    #[test]
    fn accepts_incrementing_fcnt() {
        let m = LoRaWAN10xClassA;
        let s = sample_session(5);
        let o = sample_observation(6);
        let ctx = ProtocolContext {
            observation: &o,
            session: Some(&s),
        };
        assert_eq!(m.validate_uplink(ctx).unwrap(), ProtocolDecision::Accept);
    }

    #[test]
    fn rejects_duplicate_fcnt() {
        let m = LoRaWAN10xClassA;
        let s = sample_session(10);
        let o = sample_observation(10);
        let ctx = ProtocolContext {
            observation: &o,
            session: Some(&s),
        };
        assert_eq!(
            m.validate_uplink(ctx).unwrap(),
            ProtocolDecision::RejectDuplicateFrameCounter
        );
    }
}
