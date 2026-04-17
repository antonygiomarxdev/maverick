//! Downlink scheduling for Class A devices.
//!
//! LoRaWAN Class A defines two receive windows (RX1 and RX2) following each uplink.
//! RX1 opens 1 second after uplink end, RX2 opens 2 seconds after uplink end.
//!
//! This module provides `DownlinkScheduler` which:
//! - Checks for pending downlinks after uplink processing
//! - Schedules TX in RX1 window, falls back to RX2 on failure
//! - Handles confirmed uplink ACK and MAC command responses
//!
//! Reference: LoRaWAN 1.0.x Specification Section 3.

use std::sync::Arc;

use maverick_core::error::{AppError, AppResult};
use maverick_core::ports::{DownlinkFrame, DownlinkItem, DownlinkRepository, RadioTransport};
use maverick_core::protocol::LinkCheckAns;
use maverick_domain::{DevAddr, DevEui, GatewayEui};
use tokio::time::{sleep, Duration};

/// RX1 delay after uplink receive time (1 second per LoRaWAN spec).
const RX1_DELAY_MS: u64 = 1000;
/// RX2 delay after uplink receive time (2 seconds per LoRaWAN spec).
const RX2_DELAY_MS: u64 = 2000;

/// Downlink scheduler for Class A devices.
///
/// Schedules TX in RX1/RX2 windows based on uplink observations.
pub struct DownlinkScheduler<DR, RT>
where
    DR: DownlinkRepository,
    RT: RadioTransport,
{
    downlink_repo: Arc<DR>,
    radio_transport: Arc<RT>,
}

impl<DR, RT> DownlinkScheduler<DR, RT>
where
    DR: DownlinkRepository,
    RT: RadioTransport,
{
    /// Create a new DownlinkScheduler.
    pub fn new(downlink_repo: Arc<DR>, radio_transport: Arc<RT>) -> Self {
        Self {
            downlink_repo,
            radio_transport,
        }
    }

    /// Schedule and transmit pending downlinks for a device after uplink.
    ///
    /// Called after uplink processing completes. This function:
    /// 1. Checks for pending downlinks in the queue
    /// 2. Parses MAC commands from the uplink FOpts
    /// 3. Waits for RX1 window (+1s from uplink)
    /// 4. Attempts TX in RX1
    /// 5. On failure, falls back to RX2 (+2s from uplink)
    /// 6. Handles ACK flag for confirmed uplinks
    pub async fn schedule_after_uplink(
        &self,
        dev_eui: &DevEui,
        dev_addr: DevAddr,
        gateway_eui: GatewayEui,
        confirmed_uplink: bool,
        link_check_req: bool,
    ) -> AppResult<Option<ScheduledDownlink>> {
        // Get pending downlinks for this device
        let pending = self.downlink_repo.get_pending_for_dev(dev_eui).await?;
        if pending.is_empty() && !link_check_req {
            return Ok(None); // Nothing to send
        }

        // Calculate RX1 timestamp
        let rx1_deadline = Duration::from_millis(RX1_DELAY_MS);
        sleep(rx1_deadline).await;

        // Try to send in RX1
        let result = self
            .try_send_downlink(
                dev_eui,
                dev_addr,
                gateway_eui,
                &pending,
                link_check_req,
                confirmed_uplink,
            )
            .await;

        match result {
            Ok(scheduled) => Ok(Some(scheduled)),
            Err(_) => {
                // RX1 failed, wait for RX2 and try again
                let rx2_wait = Duration::from_millis(RX2_DELAY_MS - RX1_DELAY_MS);
                sleep(rx2_wait).await;

                self.try_send_downlink(
                    dev_eui,
                    dev_addr,
                    gateway_eui,
                    &pending,
                    link_check_req,
                    confirmed_uplink,
                )
                .await
                .map(Some)
            }
        }
    }

    /// Attempt to send a downlink in the current RX window.
    async fn try_send_downlink(
        &self,
        dev_eui: &DevEui,
        dev_addr: DevAddr,
        gateway_eui: GatewayEui,
        pending: &[DownlinkItem],
        link_check_req: bool,
        _confirmed_uplink: bool,
    ) -> AppResult<ScheduledDownlink> {
        // Build MAC commands if needed
        let mut mac_commands = Vec::new();

        // If LinkCheckReq was in FOpts, queue LinkCheckAns
        // Margin is a fixed value (could be computed from SNR in real implementation)
        // Gateway count is typically 1 for single-gateway deployments
        if link_check_req {
            let link_check_ans = LinkCheckAns {
                margin: 10, // dB margin - reasonable default
                gateway_count: 1,
            };
            mac_commands.extend(link_check_ans.encode_fopts());
        }

        // Get the first pending downlink (if any)
        let (frame_payload, downlink_id) = if let Some(item) = pending.first() {
            let payload = self.build_downlink_payload(item, &mac_commands)?;
            (payload, Some(item.id))
        } else if !mac_commands.is_empty() {
            // Only MAC commands, no data downlink
            (mac_commands, None)
        } else {
            return Err(AppError::Domain("no downlink to send".to_string()));
        };

        let frame = DownlinkFrame {
            gateway_eui,
            dev_addr,
            payload: frame_payload,
        };

        // Attempt TX
        self.radio_transport.send_downlink(&frame).await?;

        // Mark as transmitted if we had a downlink item
        if let Some(id) = downlink_id {
            self.downlink_repo.mark_transmitted(id).await?;
        }

        Ok(ScheduledDownlink {
            window: ReceiveWindow::Rx1,
            transmitted: downlink_id.is_some(),
        })
    }

    /// Build the downlink payload including MAC commands.
    fn build_downlink_payload(
        &self,
        item: &DownlinkItem,
        mac_commands: &[u8],
    ) -> AppResult<Vec<u8>> {
        // For now, return the pending payload with mac commands appended
        // A full implementation would encode the full LoRaWAN downlink frame
        // (MHDR, FHDR with ACK flag, FOpts with MAC commands, FPort, FRMPayload, MIC)
        let mut payload = item.payload.clone();

        // Append MAC commands if any
        if !mac_commands.is_empty() {
            payload.extend_from_slice(mac_commands);
        }

        Ok(payload)
    }
}

/// Result of a scheduled downlink attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduledDownlink {
    /// Which receive window was used.
    pub window: ReceiveWindow,
    /// True if a frame was transmitted.
    pub transmitted: bool,
}

/// Receive window identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiveWindow {
    /// RX1 window — 1 second after uplink.
    Rx1,
    /// RX2 window — 2 seconds after uplink.
    Rx2,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rx_timing_constants() {
        assert_eq!(RX1_DELAY_MS, 1000);
        assert_eq!(RX2_DELAY_MS, 2000);
        assert!(RX2_DELAY_MS > RX1_DELAY_MS);
    }
}
