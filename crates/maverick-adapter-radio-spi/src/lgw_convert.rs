//! Convert libloragw `lgw_pkt_rx_s` structs to `UplinkObservation`.
//!
//! KEY: `lgw_pkt_rx_s.payload` contains the full LoRaWAN PHY frame INCLUDING the 4-byte MIC.
//! The split MUST happen here:
//!   wire_mic      = payload[size-4..size]       (last 4 bytes)
//!   phy_without_mic = payload[..size-4]         (everything before MIC)
//! Without this split, MIC verification in IngestUplink receives zeros and ALL frames are rejected.

use crate::lgw_bindings::lgw_pkt_rx_s;
use maverick_core::error::{AppError, AppResult};
use maverick_core::ports::UplinkObservation;
use maverick_domain::{DevAddr, GatewayEui, RegionId};

pub fn lgw_pkt_rx_to_observation(
    pkt: &lgw_pkt_rx_s,
    gateway_eui: GatewayEui,
) -> AppResult<UplinkObservation> {
    let size = pkt.size as usize;
    if size == 0 || size > 256 {
        return Err(AppError::Infrastructure(
            "invalid packet size from lgw_receive".to_string(),
        ));
    }

    let payload_ptr = pkt.payload.as_ptr();

    let full_payload = unsafe { std::slice::from_raw_parts(payload_ptr, size) };

    if size < 4 {
        return Err(AppError::Infrastructure(format!(
            "packet too small for MIC: {} bytes",
            size
        )));
    }

    let wire_mic: [u8; 4] = full_payload[size - 4..].try_into().unwrap();
    let phy_without_mic = full_payload[..size - 4].to_vec();

    let dev_addr = {
        let val = u32::from_le_bytes([
            full_payload[1],
            full_payload[2],
            full_payload[3],
            full_payload[4],
        ]);
        DevAddr(val)
    };
    let f_cnt = u16::from_le_bytes([full_payload[6], full_payload[7]]);
    let fctrl = full_payload[5];
    let fopts_len = (fctrl & 0x0F) as usize;
    let fport_idx = 8 + fopts_len;
    let f_port = if size > fport_idx {
        full_payload[fport_idx]
    } else {
        0
    };
    let payload = if size > fport_idx + 1 {
        full_payload[fport_idx + 1..size - 4].to_vec()
    } else {
        vec![]
    };

    let region = freq_to_region(pkt.freq_hz);
    let rssi = Some(pkt.rssic as i16);
    let snr = Some(pkt.snr);
    let f_ctrl = fctrl;
    let f_opts = if fopts_len > 0 {
        full_payload[8..8 + fopts_len].to_vec()
    } else {
        vec![]
    };

    Ok(UplinkObservation {
        dev_addr,
        f_cnt,
        f_port,
        payload,
        wire_mic,
        phy_without_mic,
        gateway_eui,
        region,
        rssi,
        snr,
        f_ctrl,
        f_opts,
    })
}

fn freq_to_region(freq_hz: u32) -> RegionId {
    match freq_hz {
        915_000_000..=928_000_000 => RegionId::Au915,
        923_000_000..=927_000_000 => RegionId::As923,
        867_000_000..=869_000_000 => RegionId::Eu868,
        779_000_000..=787_000_000 => RegionId::Eu433,
        470_000_000..=510_000_000 => RegionId::Us915,
        902_000_000..=928_000_000 => RegionId::Us915,
        _ => RegionId::Eu868,
    }
}
