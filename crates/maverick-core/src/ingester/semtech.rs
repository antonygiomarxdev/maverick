use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde::Deserialize;

use maverick_domain::{Eui64, Frequency, Rssi, Snr, SpreadingFactor, UplinkFrame};

use crate::error::{DomainError, Result};
use crate::use_cases::IngestUplinkCommand;

const PROTOCOL_VERSION: u8 = 0x02;
const PUSH_DATA_IDENTIFIER: u8 = 0x00;

#[derive(Debug, Clone)]
pub struct ParsedPushData {
    pub token: [u8; 2],
    pub gateway_eui: Eui64,
    pub commands: Vec<IngestUplinkCommand>,
}

#[derive(Debug, Deserialize)]
struct PushDataBody {
    #[serde(default)]
    rxpk: Vec<RxPk>,
}

#[derive(Debug, Deserialize)]
struct RxPk {
    tmst: Option<u64>,
    freq: f64,
    chan: u8,
    stat: Option<i8>,
    modu: Option<String>,
    datr: String,
    codr: Option<String>,
    rssi: i16,
    lsnr: f32,
    data: String,
}

pub fn parse_push_data(datagram: &[u8]) -> Result<ParsedPushData> {
    if datagram.len() < 12 {
        return Err(DomainError::Validation {
            field: "udp_packet",
            reason: "datagram too short for semtech udp header".to_string(),
        }
        .into());
    }

    if datagram[0] != PROTOCOL_VERSION {
        return Err(DomainError::Validation {
            field: "udp_packet.version",
            reason: format!("unsupported semtech version {}", datagram[0]),
        }
        .into());
    }

    if datagram[3] != PUSH_DATA_IDENTIFIER {
        return Err(DomainError::Validation {
            field: "udp_packet.identifier",
            reason: format!("unsupported semtech identifier {}", datagram[3]),
        }
        .into());
    }

    let token = [datagram[1], datagram[2]];
    let gateway_eui_bytes: [u8; 8] =
        datagram[4..12]
            .try_into()
            .map_err(|_| DomainError::Validation {
                field: "udp_packet.gateway_eui",
                reason: "invalid gateway eui bytes".to_string(),
            })?;
    let gateway_eui = Eui64::from(gateway_eui_bytes);

    let body: PushDataBody = serde_json::from_slice(&datagram[12..])?;
    let correlation_id = Some(format!("{:02X}{:02X}", token[0], token[1]));
    let mut commands = Vec::with_capacity(body.rxpk.len());

    for rxpk in body.rxpk {
        commands.push(rxpk_to_command(
            gateway_eui,
            rxpk,
            datagram.to_vec(),
            correlation_id.clone(),
        )?);
    }

    Ok(ParsedPushData {
        token,
        gateway_eui,
        commands,
    })
}

fn rxpk_to_command(
    gateway_eui: Eui64,
    rxpk: RxPk,
    raw_frame: Vec<u8>,
    correlation_id: Option<String>,
) -> Result<IngestUplinkCommand> {
    let payload = STANDARD
        .decode(rxpk.data)
        .map_err(|_| DomainError::Validation {
            field: "rxpk.data",
            reason: "invalid base64 payload".to_string(),
        })?;
    let (spreading_factor, bandwidth) = parse_datr(&rxpk.datr)?;
    let timestamp = rxpk.tmst.unwrap_or_default() as i64;

    let mut uplink = UplinkFrame::new(
        gateway_eui,
        payload,
        Rssi::from(rxpk.rssi),
        Snr::from(rxpk.lsnr),
        Frequency::from(mhz_to_hz(rxpk.freq)?),
        spreading_factor,
        timestamp,
        raw_frame,
    );
    uplink.metadata.channel = rxpk.chan;
    uplink.metadata.crc_status = rxpk.stat.map(|value| value as u8);
    uplink.metadata.modulation = rxpk.modu;
    uplink.metadata.code_rate = rxpk.codr;
    uplink.metadata.bandwidth = Some(bandwidth);

    Ok(IngestUplinkCommand {
        uplink,
        correlation_id,
    })
}

fn parse_datr(value: &str) -> Result<(SpreadingFactor, u32)> {
    let value = value.to_ascii_uppercase();
    let value = value
        .strip_prefix("SF")
        .ok_or_else(|| DomainError::Validation {
            field: "rxpk.datr",
            reason: "missing SF prefix".to_string(),
        })?;
    let (sf_part, bandwidth_part) =
        value
            .split_once("BW")
            .ok_or_else(|| DomainError::Validation {
                field: "rxpk.datr",
                reason: "missing BW separator".to_string(),
            })?;
    let sf = sf_part.parse::<u8>().map_err(|_| DomainError::Validation {
        field: "rxpk.datr",
        reason: format!("invalid spreading factor '{sf_part}'"),
    })?;
    let spreading_factor = SpreadingFactor::new(sf).ok_or_else(|| DomainError::Validation {
        field: "rxpk.datr",
        reason: format!("unsupported spreading factor {sf}"),
    })?;
    let bandwidth = bandwidth_part
        .parse::<u32>()
        .map_err(|_| DomainError::Validation {
            field: "rxpk.datr",
            reason: format!("invalid bandwidth '{bandwidth_part}'"),
        })?;

    Ok((spreading_factor, bandwidth * 1000))
}

fn mhz_to_hz(value: f64) -> Result<u32> {
    if value <= 0.0 {
        return Err(DomainError::Validation {
            field: "rxpk.freq",
            reason: "frequency must be positive".to_string(),
        }
        .into());
    }

    Ok((value * 1_000_000.0).round() as u32)
}

#[cfg(test)]
mod tests {
    use super::parse_push_data;

    #[test]
    fn parses_semtech_push_data_datagram() {
        let json = r#"{"rxpk":[{"tmst":123456,"freq":868.1,"chan":2,"stat":1,"modu":"LORA","datr":"SF7BW125","codr":"4/5","rssi":-32,"lsnr":5.5,"data":"AQIDBA=="}]}"#;
        let mut datagram = vec![0x02, 0xAA, 0xBB, 0x00];
        datagram.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
        datagram.extend_from_slice(json.as_bytes());

        let parsed = parse_push_data(&datagram).expect("push data must parse");
        assert_eq!(parsed.gateway_eui.as_bytes(), [1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(parsed.commands.len(), 1);
        assert_eq!(parsed.commands[0].uplink.payload, vec![1, 2, 3, 4]);
        assert_eq!(parsed.commands[0].uplink.frequency.as_hz(), 868_100_000);
        assert_eq!(parsed.commands[0].uplink.metadata.bandwidth, Some(125_000));
    }
}
