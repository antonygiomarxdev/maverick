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

//! Semtech GWMP PUSH_DATA parsing into core `UplinkObservation`.

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use maverick_core::error::{AppError, AppResult};
use maverick_core::ports::UplinkObservation;
use maverick_domain::identifiers::Eui64;
use maverick_domain::{DevAddr, GatewayEui, RegionId};
use serde::Deserialize;

const GWMP_PUSH_DATA_IDENTIFIER: u8 = 0x00;
const GWMP_HEADER_MIN_LEN: usize = 12;
const GWMP_VERSION_INDEX: usize = 0;
const GWMP_IDENTIFIER_INDEX: usize = 3;
const GWMP_GATEWAY_EUI_START: usize = 4;
const GWMP_JSON_START: usize = 12;
const LORAWAN_FHDR_DEVADDR_START: usize = 1;
const LORAWAN_FHDR_DEVADDR_END: usize = 5;
const LORAWAN_FHDR_FCTRL_INDEX: usize = 5;
const LORAWAN_FHDR_FCNT_START: usize = 6;
const LORAWAN_FHDR_FCNT_END: usize = 8;
const LORAWAN_MACPAYLOAD_MIN_LEN: usize = 9;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GwmpPacketMeta {
    pub protocol_version: u8,
    pub gateway_eui: GatewayEui,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GwmpUplinkBatch {
    pub meta: GwmpPacketMeta,
    pub observations: Vec<UplinkObservation>,
}

#[derive(Debug, Deserialize)]
struct PushDataRoot {
    #[serde(default)]
    rxpk: Vec<Rxpk>,
}

#[derive(Debug, Deserialize)]
struct Rxpk {
    data: String,
    #[serde(default)]
    freq: Option<f64>,
    #[serde(default)]
    rssi: Option<i16>,
    #[serde(default)]
    lsnr: Option<f32>,
}

pub fn parse_push_data(datagram: &[u8]) -> AppResult<GwmpUplinkBatch> {
    if datagram.len() < GWMP_HEADER_MIN_LEN {
        return Err(AppError::InvalidInput(
            "gwmp datagram too short for PUSH_DATA".to_string(),
        ));
    }
    let identifier = datagram[GWMP_IDENTIFIER_INDEX];
    if identifier != GWMP_PUSH_DATA_IDENTIFIER {
        return Err(AppError::InvalidInput(format!(
            "gwmp unsupported identifier {identifier:#04x}"
        )));
    }
    let protocol_version = datagram[GWMP_VERSION_INDEX];
    let gateway_eui = parse_gateway_eui(datagram)?;
    let body = std::str::from_utf8(&datagram[GWMP_JSON_START..])
        .map_err(|e| AppError::InvalidInput(format!("gwmp utf8 payload: {e}")))?;
    let payload: PushDataRoot = serde_json::from_str(body)
        .map_err(|e| AppError::InvalidInput(format!("gwmp json parse: {e}")))?;

    let mut observations = Vec::with_capacity(payload.rxpk.len());
    for rx in payload.rxpk {
        observations.push(rxpk_to_observation(gateway_eui, rx)?);
    }
    Ok(GwmpUplinkBatch {
        meta: GwmpPacketMeta {
            protocol_version,
            gateway_eui,
        },
        observations,
    })
}

pub fn parse_push_data_json(
    gateway_eui: GatewayEui,
    protocol_version: u8,
    json_payload: &str,
) -> AppResult<GwmpUplinkBatch> {
    let root: PushDataRoot = serde_json::from_str(json_payload)
        .map_err(|e| AppError::InvalidInput(format!("gwmp json parse: {e}")))?;
    let mut observations = Vec::with_capacity(root.rxpk.len());
    for rx in root.rxpk {
        observations.push(rxpk_to_observation(gateway_eui, rx)?);
    }
    Ok(GwmpUplinkBatch {
        meta: GwmpPacketMeta {
            protocol_version,
            gateway_eui,
        },
        observations,
    })
}

fn parse_gateway_eui(datagram: &[u8]) -> AppResult<GatewayEui> {
    let mut arr = [0_u8; 8];
    arr.copy_from_slice(&datagram[GWMP_GATEWAY_EUI_START..GWMP_JSON_START]);
    Ok(GatewayEui(Eui64(arr)))
}

fn rxpk_to_observation(gateway_eui: GatewayEui, rx: Rxpk) -> AppResult<UplinkObservation> {
    let decoded = B64
        .decode(rx.data.as_bytes())
        .map_err(|e| AppError::InvalidInput(format!("gwmp rxpk data base64: {e}")))?;
    let (dev_addr, f_cnt, f_port, payload, wire_mic, phy_without_mic, f_ctrl, f_opts) =
        parse_lorawan_payload(&decoded)?;
    Ok(UplinkObservation {
        gateway_eui,
        dev_addr,
        region: infer_region(rx.freq),
        f_cnt,
        f_port,
        payload,
        rssi: rx.rssi,
        snr: rx.lsnr,
        wire_mic,
        phy_without_mic,
        f_ctrl,
        f_opts,
    })
}

type ParsedLorawanPhy = (DevAddr, u16, u8, Vec<u8>, [u8; 4], Vec<u8>, u8, Vec<u8>);

fn parse_lorawan_payload(raw: &[u8]) -> AppResult<ParsedLorawanPhy> {
    if raw.len() < LORAWAN_MACPAYLOAD_MIN_LEN {
        return Err(AppError::InvalidInput(
            "lorawan payload too short".to_string(),
        ));
    }
    let mut dev_addr_bytes = [0_u8; 4];
    dev_addr_bytes.copy_from_slice(&raw[LORAWAN_FHDR_DEVADDR_START..LORAWAN_FHDR_DEVADDR_END]);
    let dev_addr = DevAddr(u32::from_le_bytes(dev_addr_bytes));
    let fctrl = raw[LORAWAN_FHDR_FCTRL_INDEX];
    let fopts_len = usize::from(fctrl & 0x0F);
    // f_cnt is u16 — wire value; 32-bit reconstruction happens in protocol module
    let fcnt_u16 =
        u16::from_le_bytes([raw[LORAWAN_FHDR_FCNT_START], raw[LORAWAN_FHDR_FCNT_END - 1]]);
    let fport_index = LORAWAN_FHDR_FCNT_END + fopts_len;
    if raw.len() <= fport_index {
        return Err(AppError::InvalidInput(
            "lorawan payload missing fport".to_string(),
        ));
    }
    let f_port = raw[fport_index];
    let frm_payload_start = fport_index + 1;
    if raw.len() < frm_payload_start {
        return Err(AppError::InvalidInput(
            "lorawan payload malformed".to_string(),
        ));
    }
    let mic_len = 4;
    if raw.len() < mic_len {
        return Err(AppError::InvalidInput(
            "lorawan payload too short for MIC".to_string(),
        ));
    }
    // Extract MIC (last 4 bytes) and PHY without MIC
    let mut wire_mic = [0u8; 4];
    wire_mic.copy_from_slice(&raw[raw.len() - mic_len..]);
    let phy_without_mic = raw[..raw.len() - mic_len].to_vec();

    let frm_payload_end = raw.len() - mic_len;
    let payload = if frm_payload_start < frm_payload_end {
        raw[frm_payload_start..frm_payload_end].to_vec()
    } else {
        vec![]
    };
    // Extract FOpts (bytes between FCnt and FPort)
    let fopts_start = LORAWAN_FHDR_FCNT_END;
    let fopts_end = fopts_start + fopts_len;
    let f_opts = if fopts_len > 0 {
        raw[fopts_start..fopts_end].to_vec()
    } else {
        vec![]
    };
    Ok((
        dev_addr,
        fcnt_u16,
        f_port,
        payload,
        wire_mic,
        phy_without_mic,
        fctrl,
        f_opts,
    ))
}

fn infer_region(freq_mhz: Option<f64>) -> RegionId {
    match freq_mhz {
        // AS923 first — most specific (overlaps with AU915 range)
        Some(v) if (923.0..=923.5).contains(&v) => RegionId::As923,
        // AU915 before US915 — AU915 (915–928 MHz) overlaps with US915 upper band
        Some(v) if (915.0..=928.0).contains(&v) => RegionId::Au915,
        // US915 lower band (below 915 MHz, no overlap with AU915)
        Some(v) if (902.0..915.0).contains(&v) => RegionId::Us915,
        Some(v) if (863.0..=870.0).contains(&v) => RegionId::Eu868,
        Some(v) if (433.0..=434.8).contains(&v) => RegionId::Eu433,
        _ => RegionId::Eu868,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_push_data_json_into_observation() {
        let gw = GatewayEui(Eui64([1, 2, 3, 4, 5, 6, 7, 8]));
        let body = r#"{
          "rxpk":[
            {"freq":868.1,"rssi":-57,"lsnr":5.2,"data":"QAECAwQEAAEByv66vg=="}
          ]
        }"#;
        let batch = parse_push_data_json(gw, 2, body).expect("batch");
        assert_eq!(batch.observations.len(), 1);
        assert_eq!(batch.observations[0].dev_addr.0, 0x0403_0201);
        assert!(batch.observations[0].f_cnt > 0);
        assert!(batch.observations[0].f_port > 0);
    }

    #[test]
    fn malformed_json_returns_invalid_input() {
        let gw = GatewayEui(Eui64([9; 8]));
        let err = parse_push_data_json(gw, 2, "{not-json").expect_err("must fail");
        assert!(matches!(err, AppError::InvalidInput(_)));
    }

    #[test]
    fn parses_burst_multiple_rxpk_entries() {
        let gw = GatewayEui(Eui64([1; 8]));
        let body = r#"{
          "rxpk":[
            {"freq":868.1,"rssi":-57,"lsnr":5.2,"data":"QAECAwQEAAEByv66vg=="},
            {"freq":868.3,"rssi":-60,"lsnr":4.8,"data":"QAECAwQEAAEByv66vg=="}
          ]
        }"#;
        let batch = parse_push_data_json(gw, 2, body).expect("batch");
        assert_eq!(batch.observations.len(), 2);
    }

    #[test]
    fn infer_region_au915_not_shadowed_by_us915() {
        // 916.8 MHz is AU915 uplink channel 8
        let gw = GatewayEui(Eui64([1; 8]));
        let body =
            r#"{"rxpk":[{"freq":916.8,"rssi":-70,"lsnr":6.0,"data":"QAECAwQEAAEByv66vg=="}]}"#;
        let batch = parse_push_data_json(gw, 2, body).expect("batch");
        assert_eq!(batch.observations[0].region, RegionId::Au915);
    }

    #[test]
    fn infer_region_as923_identified() {
        // 923.2 MHz is AS923 channel
        let gw = GatewayEui(Eui64([1; 8]));
        let body =
            r#"{"rxpk":[{"freq":923.2,"rssi":-70,"lsnr":6.0,"data":"QAECAwQEAAEByv66vg=="}]}"#;
        let batch = parse_push_data_json(gw, 2, body).expect("batch");
        assert_eq!(batch.observations[0].region, RegionId::As923);
    }

    #[test]
    fn infer_region_us915_below_915() {
        // 903.9 MHz is US915 channel 7
        let gw = GatewayEui(Eui64([1; 8]));
        let body =
            r#"{"rxpk":[{"freq":903.9,"rssi":-70,"lsnr":6.0,"data":"QAECAwQEAAEByv66vg=="}]}"#;
        let batch = parse_push_data_json(gw, 2, body).expect("batch");
        assert_eq!(batch.observations[0].region, RegionId::Us915);
    }

    // ========================================================================
    // UDP adapter MIC extraction integration tests
    // ========================================================================
    // These tests verify that parse_lorawan_payload correctly extracts:
    // - wire_mic: last 4 bytes of raw PHY payload
    // - phy_without_mic: raw[..len-4] for MIC computation in ingest
    // - DevAddr, FCnt, FPort, payload fields

    /// Construct a known LoRaWAN 1.0.x uplink frame and verify MIC extraction.
    /// MHDR (1 byte): MType=0b_010_00_00 (unconfirmed uplink), Major=00
    #[test]
    fn full_pipeline_valid_frame_with_mic() {
        // Manual LoRaWAN 1.0.x uplink frame construction:
        // MHDR: 0x40 (unconfirmed uplink, major=00)
        // DevAddr: 0x04_03_02_01 (LE = 0x01, 0x02, 0x03, 0x04)
        // FCtrl: 0x00 (no FOpts, ADR=0, ACK=0, RFU=0)
        // FCnt: 0x00_01 (LE = 0x01, 0x00)
        // FPort: 0x01
        // FRMPayload: 0xAA, 0xBB, 0xCC (3 bytes)
        // MIC: computed from B0 || PHY_without_MIC
        let raw = vec![
            0x40, // MHDR
            0x01, 0x02, 0x03, 0x04, // DevAddr (LE)
            0x00, // FCtrl
            0x01, 0x00, // FCnt = 1 (LE)
            0x01, // FPort
            0xAA, 0xBB, 0xCC, // FRMPayload
            0x00, 0x00, 0x00, 0x00, // MIC (placeholder)
        ];
        let base64_frame = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &raw);

        let gw = GatewayEui(Eui64([1; 8]));
        let body = format!(
            r#"{{"rxpk":[{{"freq":868.1,"rssi":-57,"lsnr":5.2,"data":"{}"}}]}}"#,
            base64_frame
        );
        let batch = parse_push_data_json(gw, 2, &body).expect("batch");

        assert_eq!(batch.observations.len(), 1);
        let obs = &batch.observations[0];

        // Verify DevAddr extracted correctly
        assert_eq!(obs.dev_addr.0, 0x0403_0201);

        // Verify FCnt wire value
        assert_eq!(obs.f_cnt, 0x0001);

        // Verify FPort
        assert_eq!(obs.f_port, 0x01);

        // Verify payload
        assert_eq!(obs.payload, vec![0xAA, 0xBB, 0xCC]);

        // Verify wire_mic is last 4 bytes
        assert_eq!(obs.wire_mic, [0x00, 0x00, 0x00, 0x00]);

        // Verify phy_without_mic has correct length (raw.len() - 4 = 11 - 4 = 7)
        assert_eq!(
            obs.phy_without_mic.len(),
            raw.len() - 4,
            "phy_without_mic.len() must equal raw.len() - 4"
        );
    }

    /// Verify wire_mic equals exact last 4 bytes and phy_without_mic excludes MIC.
    #[test]
    fn mic_extraction_from_known_frame() {
        // Construct a frame where we know the last 4 bytes
        let known_mic = [0xDE, 0xAD, 0xBE, 0xEF];
        let raw = vec![
            0x40, // MHDR
            0x01,
            0x02,
            0x03,
            0x04, // DevAddr
            0x00, // FCtrl
            0x01,
            0x00, // FCnt
            0x01, // FPort
            0xAA,
            0xBB, // FRMPayload (2 bytes)
            known_mic[0],
            known_mic[1],
            known_mic[2],
            known_mic[3], // MIC
        ];
        let base64_frame = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &raw);

        let gw = GatewayEui(Eui64([1; 8]));
        let body = format!(
            r#"{{"rxpk":[{{"freq":868.1,"rssi":-57,"lsnr":5.2,"data":"{}"}}]}}"#,
            base64_frame
        );
        let batch = parse_push_data_json(gw, 2, &body).expect("batch");

        assert_eq!(batch.observations[0].wire_mic, known_mic);

        // Verify phy_without_mic does NOT contain MIC bytes
        assert!(
            !batch.observations[0].phy_without_mic.ends_with(&known_mic),
            "phy_without_mic must not contain MIC bytes"
        );
    }

    /// Verify `phy_without_mic.len() == raw.len() - 4` for a frame with MIC.
    #[test]
    fn phy_without_mic_correct_length() {
        // Frame with known length
        let raw = vec![
            0x40, // MHDR
            0x01, 0x02, 0x03, 0x04, // DevAddr
            0x00, // FCtrl
            0x01, 0x00, // FCnt
            0x01, // FPort
            0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, // FRMPayload (6 bytes)
            0x11, 0x22, 0x33, 0x44, // MIC (4 bytes)
        ];
        // Total: 1 + 4 + 1 + 2 + 1 + 6 + 4 = 19 bytes
        assert_eq!(raw.len(), 19);

        let base64_frame = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &raw);

        let gw = GatewayEui(Eui64([1; 8]));
        let body = format!(
            r#"{{"rxpk":[{{"freq":868.1,"rssi":-57,"lsnr":5.2,"data":"{}"}}]}}"#,
            base64_frame
        );
        let batch = parse_push_data_json(gw, 2, &body).expect("batch");

        let obs = &batch.observations[0];
        assert_eq!(
            obs.phy_without_mic.len(),
            15,
            "phy_without_mic.len() must be 19 - 4 = 15"
        );
    }
}
