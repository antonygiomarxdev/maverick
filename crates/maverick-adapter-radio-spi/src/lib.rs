//! SPI / SX130x concentrator uplink adapter (feature `spi`).

#[cfg(feature = "spi")]
mod ingress_identity;
#[cfg(feature = "spi")]
mod spi_uplink;

#[cfg(feature = "spi")]
pub use ingress_identity::SpiConcentratorIngressBackend;
#[cfg(feature = "spi")]
pub use spi_uplink::SpiUplinkSource;
