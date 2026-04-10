//! First-party UDP radio adapter skeleton: implements `RadioTransport` port without coupling core.

use async_trait::async_trait;
use maverick_core::error::{AppError, AppResult};
use maverick_core::ports::{DownlinkFrame, RadioTransport};

/// Placeholder adapter until Semtech UDP parsing is wired.
pub struct UdpRadioStub;

#[async_trait]
impl RadioTransport for UdpRadioStub {
    async fn send_downlink(&self, _frame: &DownlinkFrame) -> AppResult<()> {
        Err(AppError::Infrastructure(
            "UDP radio adapter not yet implemented".to_string(),
        ))
    }
}
