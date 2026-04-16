//! Uplink ingress backend identity for SPI / concentrator path.

use maverick_core::ports::{UplinkBackendKind, UplinkIngressBackend};

/// SPI-attached SX1302/SX1303 (via libloragw when integrated).
#[derive(Debug, Default, Clone, Copy)]
pub struct SpiConcentratorIngressBackend;

impl UplinkIngressBackend for SpiConcentratorIngressBackend {
    fn kind(&self) -> UplinkBackendKind {
        UplinkBackendKind::ConcentratorSpi
    }

    fn id(&self) -> &'static str {
        "sx130x_spi"
    }
}
