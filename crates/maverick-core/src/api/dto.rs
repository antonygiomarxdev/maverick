use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde::{Deserialize, Serialize};

use maverick_domain::{
    AppKey, Device, DeviceClass, DeviceState, Downlink, DownlinkPriority, Eui64, Frequency,
    Gateway, GatewayStatus, NwkKey, SpreadingFactor,
};

use crate::ports::{DownlinkState, QueuedDownlink};
use crate::use_cases::{CreateDeviceCommand, DownlinkDraft, UpdateDeviceCommand};
use crate::{DomainError, Result};

#[derive(Debug, Deserialize)]
pub struct CreateDeviceRequestDto {
    pub dev_eui: String,
    pub app_eui: String,
    pub app_key: String,
    pub nwk_key: String,
    pub class: Option<String>,
}

impl CreateDeviceRequestDto {
    pub fn into_command(self) -> Result<CreateDeviceCommand> {
        Ok(CreateDeviceCommand {
            dev_eui: parse_eui64("dev_eui", &self.dev_eui)?,
            app_eui: parse_eui64("app_eui", &self.app_eui)?,
            app_key: parse_app_key("app_key", &self.app_key)?,
            nwk_key: parse_nwk_key("nwk_key", &self.nwk_key)?,
            class: parse_device_class(self.class.as_deref())?,
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct PatchDeviceRequestDto {
    pub app_eui: Option<String>,
    pub app_key: Option<String>,
    pub nwk_key: Option<String>,
    pub class: Option<String>,
    pub state: Option<String>,
}

impl PatchDeviceRequestDto {
    pub fn into_command(self, dev_eui: Eui64) -> Result<UpdateDeviceCommand> {
        Ok(UpdateDeviceCommand {
            dev_eui,
            app_eui: self
                .app_eui
                .as_deref()
                .map(|value| parse_eui64("app_eui", value))
                .transpose()?,
            app_key: self
                .app_key
                .as_deref()
                .map(|value| parse_app_key("app_key", value))
                .transpose()?,
            nwk_key: self
                .nwk_key
                .as_deref()
                .map(|value| parse_nwk_key("nwk_key", value))
                .transpose()?,
            class: self
                .class
                .as_deref()
                .map(parse_device_class_str)
                .transpose()?,
            state: self
                .state
                .as_deref()
                .map(parse_device_state_str)
                .transpose()?,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct DeviceResponseDto {
    pub dev_eui: String,
    pub app_eui: String,
    pub app_key: String,
    pub nwk_key: String,
    pub class: String,
    pub state: String,
    pub f_cnt_up: u32,
    pub f_cnt_down: u32,
}

impl From<Device> for DeviceResponseDto {
    fn from(device: Device) -> Self {
        Self {
            dev_eui: encode_hex(&device.dev_eui.as_bytes()),
            app_eui: encode_hex(&device.app_eui.as_bytes()),
            app_key: STANDARD.encode(device.keys.app_key.as_bytes()),
            nwk_key: STANDARD.encode(device.keys.nwk_key.as_bytes()),
            class: match device.class {
                DeviceClass::ClassA => "ClassA",
                DeviceClass::ClassB => "ClassB",
                DeviceClass::ClassC => "ClassC",
            }
            .to_string(),
            state: match device.state {
                DeviceState::Init => "Init",
                DeviceState::JoinPending => "JoinPending",
                DeviceState::Active => "Active",
                DeviceState::Sleep => "Sleep",
                DeviceState::Dead => "Dead",
            }
            .to_string(),
            f_cnt_up: device.f_cnt_up,
            f_cnt_down: device.f_cnt_down,
        }
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct GatewayListQueryDto {
    pub status: Option<String>,
}

impl GatewayListQueryDto {
    pub fn status_filter(&self) -> Result<Option<GatewayStatus>> {
        self.status.as_deref().map(parse_gateway_status_str).transpose()
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct DownlinkListQueryDto {
    pub state: Option<String>,
    pub limit: Option<usize>,
}

impl DownlinkListQueryDto {
    pub fn state_filter(&self) -> Result<Option<DownlinkState>> {
        self.state.as_deref().map(parse_downlink_state_str).transpose()
    }

    pub fn limit_or_default(&self) -> usize {
        self.limit.unwrap_or(50).clamp(1, 200)
    }
}

#[derive(Debug, Serialize)]
pub struct GatewayLocationResponseDto {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct GatewayResponseDto {
    pub gateway_eui: String,
    pub status: String,
    pub location: Option<GatewayLocationResponseDto>,
    pub tx_frequency: Option<u32>,
    pub rx_temperature: Option<f32>,
    pub tx_temperature: Option<f32>,
    pub platform: Option<String>,
    pub bridge_ip: Option<String>,
    pub last_seen: Option<i64>,
}

impl From<Gateway> for GatewayResponseDto {
    fn from(gateway: Gateway) -> Self {
        Self {
            gateway_eui: encode_hex(&gateway.gateway_eui.as_bytes()),
            status: gateway_status_name(gateway.status).to_string(),
            location: gateway.location.map(|location| GatewayLocationResponseDto {
                latitude: location.latitude,
                longitude: location.longitude,
                altitude: location.altitude,
            }),
            tx_frequency: gateway.tx_frequency,
            rx_temperature: gateway.rx_temperature,
            tx_temperature: gateway.tx_temperature,
            platform: gateway.platform,
            bridge_ip: gateway.bridge_ip,
            last_seen: gateway.last_seen,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateDownlinkRequestDto {
    pub gateway_eui: Option<String>,
    pub payload: String,
    pub f_port: u8,
    pub frequency_hz: u32,
    pub spreading_factor: u8,
    pub frame_counter: u32,
    pub priority: Option<String>,
    pub scheduled_at: Option<i64>,
}

impl CreateDownlinkRequestDto {
    pub fn into_draft(self, dev_eui: Eui64) -> Result<DownlinkDraft> {
        if self.f_port > 223 {
            return Err(DomainError::Validation {
                field: "f_port",
                reason: "f_port must be between 0 and 223".to_string(),
            }
            .into());
        }

        let payload = decode_base64_bytes("payload", &self.payload, 255)?;
        let spreading_factor =
            SpreadingFactor::new(self.spreading_factor).ok_or_else(|| DomainError::Validation {
                field: "spreading_factor",
                reason: "spreading_factor must be between 7 and 12".to_string(),
            })?;

        Ok(DownlinkDraft {
            payload,
            f_port: self.f_port,
            dev_eui,
            frequency: Frequency::new(self.frequency_hz),
            spreading_factor,
            timestamp: unix_timestamp(),
            frame_counter: self.frame_counter,
            priority: parse_downlink_priority(self.priority.as_deref())?,
            scheduled_at: self.scheduled_at,
        })
    }

    pub fn into_domain(self, dev_eui: Eui64) -> Result<Downlink> {
        let gateway_eui = self.gateway_eui.clone().ok_or_else(|| DomainError::Validation {
            field: "gateway_eui",
            reason: "gateway_eui is required when no automatic selector is applied".to_string(),
        })?;
        self.into_domain_with_gateway(dev_eui, parse_eui64("gateway_eui", &gateway_eui)?)
    }

    pub fn into_domain_with_gateway(self, dev_eui: Eui64, gateway_eui: Eui64) -> Result<Downlink> {
        if self.f_port > 223 {
            return Err(DomainError::Validation {
                field: "f_port",
                reason: "f_port must be between 0 and 223".to_string(),
            }
            .into());
        }

        let payload = decode_base64_bytes("payload", &self.payload, 255)?;
        let spreading_factor =
            SpreadingFactor::new(self.spreading_factor).ok_or_else(|| DomainError::Validation {
                field: "spreading_factor",
                reason: "spreading_factor must be between 7 and 12".to_string(),
            })?;

        let mut downlink = Downlink::new(
            payload,
            self.f_port,
            dev_eui,
            gateway_eui,
            Frequency::new(self.frequency_hz),
            spreading_factor,
            unix_timestamp(),
            self.frame_counter,
        )
        .with_priority(parse_downlink_priority(self.priority.as_deref())?);
        downlink.scheduled_at = self.scheduled_at;
        Ok(downlink)
    }
}

#[derive(Debug, Serialize)]
pub struct DownlinkEnqueueResponseDto {
    pub downlink_id: i64,
    pub status: &'static str,
}

#[derive(Debug, Serialize)]
pub struct DownlinkResponseDto {
    pub downlink_id: i64,
    pub dev_eui: String,
    pub gateway_eui: String,
    pub payload: String,
    pub f_port: u8,
    pub frequency_hz: u32,
    pub spreading_factor: u8,
    pub frame_counter: u32,
    pub priority: String,
    pub state: String,
    pub attempt_count: u32,
    pub scheduled_at: Option<i64>,
    pub sent_at: Option<i64>,
    pub last_error: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<QueuedDownlink> for DownlinkResponseDto {
    fn from(value: QueuedDownlink) -> Self {
        Self {
            downlink_id: value.id,
            dev_eui: encode_hex(&value.downlink.dev_eui.as_bytes()),
            gateway_eui: encode_hex(&value.downlink.gateway_eui.as_bytes()),
            payload: STANDARD.encode(&value.downlink.payload),
            f_port: value.downlink.f_port,
            frequency_hz: value.downlink.frequency.as_hz(),
            spreading_factor: value.downlink.spreading_factor.0,
            frame_counter: value.downlink.frame_counter,
            priority: downlink_priority_name(value.downlink.priority).to_string(),
            state: downlink_state_name(value.state).to_string(),
            attempt_count: value.attempt_count,
            scheduled_at: value.downlink.scheduled_at,
            sent_at: value.sent_at,
            last_error: value.last_error,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

pub fn parse_path_dev_eui(dev_eui: &str) -> Result<Eui64> {
    parse_eui64("dev_eui", dev_eui)
}

fn parse_eui64(field: &'static str, value: &str) -> Result<Eui64> {
    let bytes = decode_hex_fixed::<8>(field, value)?;
    Ok(Eui64::from(bytes))
}

fn parse_app_key(field: &'static str, value: &str) -> Result<AppKey> {
    let bytes = decode_base64_fixed::<16>(field, value)?;
    Ok(AppKey::from(bytes))
}

fn parse_nwk_key(field: &'static str, value: &str) -> Result<NwkKey> {
    let bytes = decode_base64_fixed::<16>(field, value)?;
    Ok(NwkKey::from(bytes))
}

fn parse_device_class(value: Option<&str>) -> Result<DeviceClass> {
    value
        .map(parse_device_class_str)
        .transpose()
        .map(|value| value.unwrap_or(DeviceClass::ClassA))
}

fn parse_device_class_str(value: &str) -> Result<DeviceClass> {
    match value {
        "ClassA" => Ok(DeviceClass::ClassA),
        "ClassB" => Ok(DeviceClass::ClassB),
        "ClassC" => Ok(DeviceClass::ClassC),
        _ => Err(DomainError::Validation {
            field: "class",
            reason: format!("unsupported class '{value}'"),
        }
        .into()),
    }
}

fn parse_device_state_str(value: &str) -> Result<DeviceState> {
    match value {
        "Init" => Ok(DeviceState::Init),
        "JoinPending" => Ok(DeviceState::JoinPending),
        "Active" => Ok(DeviceState::Active),
        "Sleep" => Ok(DeviceState::Sleep),
        "Dead" => Ok(DeviceState::Dead),
        _ => Err(DomainError::Validation {
            field: "state",
            reason: format!("unsupported state '{value}'"),
        }
        .into()),
    }
}

fn parse_gateway_status_str(value: &str) -> Result<GatewayStatus> {
    match value {
        "Online" => Ok(GatewayStatus::Online),
        "Offline" => Ok(GatewayStatus::Offline),
        "Timeout" => Ok(GatewayStatus::Timeout),
        _ => Err(DomainError::Validation {
            field: "status",
            reason: "unsupported status, expected Online|Offline|Timeout".to_string(),
        }
        .into()),
    }
}

fn parse_downlink_priority(value: Option<&str>) -> Result<DownlinkPriority> {
    match value.unwrap_or("Normal") {
        "Low" => Ok(DownlinkPriority::Low),
        "Normal" => Ok(DownlinkPriority::Normal),
        "High" => Ok(DownlinkPriority::High),
        "Critical" => Ok(DownlinkPriority::Critical),
        _ => Err(DomainError::Validation {
            field: "priority",
            reason: "unsupported priority, expected Low|Normal|High|Critical".to_string(),
        }
        .into()),
    }
}

fn parse_downlink_state_str(value: &str) -> Result<DownlinkState> {
    match value {
        "Queued" => Ok(DownlinkState::Queued),
        "Scheduled" => Ok(DownlinkState::Scheduled),
        "Sent" => Ok(DownlinkState::Sent),
        "Failed" => Ok(DownlinkState::Failed),
        _ => Err(DomainError::Validation {
            field: "state",
            reason: "unsupported state, expected Queued|Scheduled|Sent|Failed".to_string(),
        }
        .into()),
    }
}

fn decode_hex_fixed<const N: usize>(field: &'static str, value: &str) -> Result<[u8; N]> {
    let normalized: String = value
        .chars()
        .filter(|ch| *ch != ':' && *ch != '-')
        .collect();
    if normalized.len() != N * 2 {
        return Err(DomainError::Validation {
            field,
            reason: format!("expected {} hex chars", N * 2),
        }
        .into());
    }

    let mut out = [0u8; N];
    for (index, chunk) in normalized.as_bytes().chunks(2).enumerate() {
        let chunk = std::str::from_utf8(chunk).map_err(|_| DomainError::Validation {
            field,
            reason: "invalid utf8 in hex value".to_string(),
        })?;
        out[index] = u8::from_str_radix(chunk, 16).map_err(|_| DomainError::Validation {
            field,
            reason: format!("invalid hex byte '{chunk}'"),
        })?;
    }

    Ok(out)
}

fn decode_base64_fixed<const N: usize>(field: &'static str, value: &str) -> Result<[u8; N]> {
    let decoded = STANDARD
        .decode(value)
        .map_err(|_| DomainError::Validation {
            field,
            reason: "invalid base64 payload".to_string(),
        })?;

    if decoded.len() != N {
        return Err(DomainError::Validation {
            field,
            reason: format!("expected {N} decoded bytes"),
        }
        .into());
    }

    let mut out = [0u8; N];
    out.copy_from_slice(&decoded);
    Ok(out)
}

fn decode_base64_bytes(field: &'static str, value: &str, max_len: usize) -> Result<Vec<u8>> {
    let decoded = STANDARD
        .decode(value)
        .map_err(|_| DomainError::Validation {
            field,
            reason: "invalid base64 payload".to_string(),
        })?;
    if decoded.is_empty() {
        return Err(DomainError::Validation {
            field,
            reason: "payload must not be empty".to_string(),
        }
        .into());
    }
    if decoded.len() > max_len {
        return Err(DomainError::Validation {
            field,
            reason: format!("payload exceeds maximum size of {max_len} bytes"),
        }
        .into());
    }
    Ok(decoded)
}

fn downlink_priority_name(value: DownlinkPriority) -> &'static str {
    match value {
        DownlinkPriority::Low => "Low",
        DownlinkPriority::Normal => "Normal",
        DownlinkPriority::High => "High",
        DownlinkPriority::Critical => "Critical",
    }
}

fn gateway_status_name(value: GatewayStatus) -> &'static str {
    match value {
        GatewayStatus::Online => "Online",
        GatewayStatus::Offline => "Offline",
        GatewayStatus::Timeout => "Timeout",
    }
}

fn downlink_state_name(value: DownlinkState) -> &'static str {
    match value {
        DownlinkState::Queued => "Queued",
        DownlinkState::Scheduled => "Scheduled",
        DownlinkState::Sent => "Sent",
        DownlinkState::Failed => "Failed",
    }
}

fn unix_timestamp() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

fn encode_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(output, "{:02X}", byte);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::{parse_path_dev_eui, CreateDeviceRequestDto};

    #[test]
    fn parses_hex_dev_eui() {
        let dev_eui = parse_path_dev_eui("0102030405060708").expect("hex must parse");
        assert_eq!(dev_eui.as_bytes(), [1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn create_device_request_parses_hex_and_base64() {
        let request = CreateDeviceRequestDto {
            dev_eui: "0102030405060708".to_string(),
            app_eui: "0807060504030201".to_string(),
            app_key: "AQEBAQEBAQEBAQEBAQEBAQ==".to_string(),
            nwk_key: "AgICAgICAgICAgICAgICAg==".to_string(),
            class: Some("ClassB".to_string()),
        };

        let command = request.into_command().expect("request must parse");
        assert_eq!(command.dev_eui.as_bytes(), [1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(command.app_eui.as_bytes(), [8, 7, 6, 5, 4, 3, 2, 1]);
        assert_eq!(command.app_key.as_bytes(), [1u8; 16]);
        assert_eq!(command.nwk_key.as_bytes(), [2u8; 16]);
    }
}
