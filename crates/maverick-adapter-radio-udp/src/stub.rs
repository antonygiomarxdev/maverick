//! Placeholder adapter for compositions that do not configure UDP yet.

use async_trait::async_trait;
use maverick_core::error::{AppError, AppResult};
use maverick_core::ports::{DownlinkFrame, RadioTransport};

const STUB_MESSAGE: &str = "UDP radio adapter not configured (use UdpDownlinkTransport)";

/// Explicit no-op / fail-fast adapter for wiring that expects a [`RadioTransport`] handle.
pub struct UdpRadioStub;

#[async_trait]
impl RadioTransport for UdpRadioStub {
    async fn send_downlink(&self, _frame: &DownlinkFrame) -> AppResult<()> {
        Err(AppError::Infrastructure(STUB_MESSAGE.to_string()))
    }
}
