use async_trait::async_trait;
use maverick_domain::{DevAddr, GatewayEui, RegionId};

use crate::error::AppResult;

/// Normalized uplink observation entering the kernel from any radio adapter.
#[derive(Debug, Clone, PartialEq)]
pub struct UplinkObservation {
    pub gateway_eui: GatewayEui,
    pub dev_addr: DevAddr,
    pub region: RegionId,
    /// Wire-level 16-bit frame counter; reconstruction to 32-bit happens in the protocol module.
    pub f_cnt: u16,
    pub f_port: u8,
    pub payload: Vec<u8>,
    pub rssi: Option<i16>,
    pub snr: Option<f32>,
    /// Raw MIC bytes (last 4 bytes of the PHY payload); preserved from parser for MIC verification.
    pub wire_mic: [u8; 4],
    /// PHY payload excluding the trailing 4 MIC bytes; used by MIC verifier.
    pub phy_without_mic: Vec<u8>,
    /// Frame control byte from FHDR; encodes FOpts length in lower 4 bits.
    pub f_ctrl: u8,
    /// FOpts bytes (MAC commands); empty if FOptsLen=0.
    pub f_opts: Vec<u8>,
}

/// Downlink command to be encoded and sent by a concrete transport adapter.
#[derive(Debug, Clone, PartialEq)]
pub struct DownlinkFrame {
    pub gateway_eui: GatewayEui,
    pub dev_addr: DevAddr,
    pub payload: Vec<u8>,
}

/// Inbound radio path (receive observations / stats). Outbound scheduling is separate in full LNS.
#[async_trait]
pub trait RadioTransport: Send + Sync {
    /// Adapter-specific: e.g. bind UDP and forward parsed observations to core.
    async fn send_downlink(&self, frame: &DownlinkFrame) -> AppResult<()>;
}
