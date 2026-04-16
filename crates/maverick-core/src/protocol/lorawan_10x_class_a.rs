use maverick_domain::{DeviceClass, LoRaWANVersion, RegionId};

use crate::error::AppResult;
use crate::protocol::{ProtocolCapability, ProtocolContext, ProtocolDecision};

/// Error returned by `extend_fcnt` when a frame must be rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FcntError {
    /// Wire FCnt falls within the gap window below session counter — frame is a duplicate or replay.
    Duplicate,
    /// Gap between reconstructed FCnt and session counter exceeds MAX_FCNT_GAP (16384).
    GapExceeded,
}

/// LoRaWAN 1.0.x Class A policy module (v1 baseline regions).
pub struct LoRaWAN10xClassA;

impl LoRaWAN10xClassA {
    fn region_supported(region: RegionId) -> bool {
        matches!(
            region,
            RegionId::Eu868 | RegionId::Us915 | RegionId::Au915 | RegionId::As923 | RegionId::Eu433
        )
    }

    /// LoRaWAN 1.0.x §4.3.1.5 — extend 16-bit wire FCnt to 32-bit server counter.
    ///
    /// Returns `Ok(reconstructed_fcnt)` if the frame should be processed.
    /// Returns `Err(FcntError::Duplicate)` if the frame counter is within the replay window.
    /// Returns `Err(FcntError::GapExceeded)` if the gap exceeds `MAX_FCNT_GAP`.
    pub fn extend_fcnt(wire_u16: u16, session_fcnt: u32) -> Result<u32, FcntError> {
        const MAX_FCNT_GAP: u32 = 16384; // LoRaWAN spec §4.3.1.5

        let candidate_low = (session_fcnt & 0xFFFF_0000) | u32::from(wire_u16);
        let candidate_high = candidate_low.wrapping_add(0x1_0000);

        if candidate_low > session_fcnt {
            // Normal forward progress — no rollover needed
            Ok(candidate_low)
        } else if session_fcnt.wrapping_sub(candidate_low) <= MAX_FCNT_GAP {
            // candidate_low <= session_fcnt AND within gap window → duplicate/replay
            Err(FcntError::Duplicate)
        } else if candidate_high.wrapping_sub(session_fcnt) <= MAX_FCNT_GAP {
            // 16-bit rollover: low candidate is in the past but high candidate is close enough
            Ok(candidate_high)
        } else {
            // Gap too large in both directions
            Err(FcntError::GapExceeded)
        }
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
        // FCnt 32-bit reconstruction (LoRaWAN spec §4.3.1.5)
        match Self::extend_fcnt(obs.f_cnt, session.uplink_frame_counter) {
            Err(FcntError::Duplicate) => return Ok(ProtocolDecision::RejectDuplicateFrameCounter),
            Err(FcntError::GapExceeded) => return Ok(ProtocolDecision::RejectFcntGapExceeded),
            Ok(_reconstructed) => {} // Accept; IngestUplink::execute will use the reconstructed value
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
            application_id: None,
            nwk_s_key: [0u8; 16],
            app_s_key: [0u8; 16],
        }
    }

    fn sample_observation(fc: u16) -> UplinkObservation {
        UplinkObservation {
            gateway_eui: maverick_domain::GatewayEui(Eui64([8; 8])),
            dev_addr: DevAddr(0x01_02_03_04),
            region: RegionId::Eu868,
            f_cnt: fc,
            f_port: 1,
            payload: vec![0x01],
            rssi: None,
            snr: None,
            wire_mic: [0u8; 4],
            phy_without_mic: vec![],
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

    #[test]
    fn extend_fcnt_no_rollover() {
        assert_eq!(
            LoRaWAN10xClassA::extend_fcnt(0x0010, 0x0000_0005),
            Ok(0x0000_0010)
        );
    }

    #[test]
    fn extend_fcnt_rollover_at_16bit_boundary() {
        assert_eq!(
            LoRaWAN10xClassA::extend_fcnt(0x0001, 0x0000_FFFE),
            Ok(0x0001_0001)
        );
    }

    #[test]
    fn extend_fcnt_duplicate_rejected() {
        assert_eq!(
            LoRaWAN10xClassA::extend_fcnt(0x0005, 0x0000_0010),
            Err(FcntError::Duplicate)
        );
    }

    #[test]
    fn extend_fcnt_gap_exceeded_rejected() {
        // Gap of 20000 > MAX_FCNT_GAP 16384
        assert_eq!(
            LoRaWAN10xClassA::extend_fcnt(0x9999, 0x0001_0000),
            Err(FcntError::GapExceeded)
        );
    }
}
