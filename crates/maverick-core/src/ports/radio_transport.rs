use async_trait::async_trait;
use maverick_domain::{DevAddr, GatewayEui, RegionId};

use crate::error::AppResult;

/// Normalized uplink observation entering the kernel from any radio adapter.
#[derive(Debug, Clone, PartialEq)]
pub struct UplinkObservation {
    pub gateway_eui: GatewayEui,
    pub dev_addr: DevAddr,
    pub region: RegionId,
    pub f_cnt: u32,
    pub f_port: u8,
    pub payload: Vec<u8>,
    pub rssi: Option<i16>,
    pub snr: Option<f32>,
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
