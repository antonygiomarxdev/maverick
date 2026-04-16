//! Radio-agnostic uplink ingestion (hexagonal port). Implementations: GWMP/UDP, SPI HAL, etc.

use async_trait::async_trait;

use super::radio_transport::UplinkObservation;
use crate::error::AppResult;

/// Result of one `next_batch` poll: either idle (no datagram before timeout) or observations from one datagram.
///
/// A GWMP datagram may parse successfully but yield zero `rxpk` rows — that is **`Observations([])`**, not `Idle`.
#[derive(Debug, Clone, PartialEq)]
pub enum UplinkReceive {
    /// No UDP datagram (or SPI equivalent idle) before the adapter read timeout.
    Idle,
    /// One inbound datagram produced this vector (may be empty if the frame contained no uplinks).
    Observations(Vec<UplinkObservation>),
}

#[async_trait]
pub trait UplinkSource: Send + Sync {
    /// Blocking-style poll: wait up to the adapter's read timeout, then return either idle or a batch.
    async fn next_batch(&self) -> AppResult<UplinkReceive>;
}
