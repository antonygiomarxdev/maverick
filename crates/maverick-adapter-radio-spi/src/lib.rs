//! SPI / SX130x concentrator uplink adapter (feature `spi`).

mod lgw_bindings;

#[cfg(feature = "spi")]
mod ingress_identity;
#[cfg(feature = "spi")]
mod lgw_convert;
#[cfg(feature = "spi")]
mod lgw_init;
#[cfg(feature = "spi")]
mod spi_uplink;

#[cfg(feature = "spi")]
pub use ingress_identity::SpiConcentratorIngressBackend;
#[cfg(feature = "spi")]
pub use spi_uplink::SpiUplinkSource;
