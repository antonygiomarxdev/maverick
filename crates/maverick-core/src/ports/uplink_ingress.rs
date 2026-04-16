//! Uplink ingestion backend identity (hexagonal boundary; implementations live in adapters).

use serde::{Deserialize, Serialize};

/// Wire-format identifier for the active uplink path (extensible without breaking JSON).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UplinkBackendKind {
    /// Semtech GWMP `PUSH_DATA` over UDP (typical packet forwarder → edge on same host).
    GwmpUdp,
    /// Direct SX1302/SX1303 SPI concentrator (libloragw / HAL).
    ConcentratorSpi,
}

/// Implemented by concrete radio ingress adapters (UDP GWMP today; SPI/USB later).
pub trait UplinkIngressBackend: Send + Sync {
    fn kind(&self) -> UplinkBackendKind;

    fn id(&self) -> &'static str;
}
