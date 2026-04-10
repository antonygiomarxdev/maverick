//! Minimal UDP downlink: sends raw payload datagrams to a configured gateway address.

use std::net::SocketAddr;

use async_trait::async_trait;
use maverick_core::error::{AppError, AppResult};
use maverick_core::ports::{DownlinkFrame, RadioTransport};
use tokio::net::UdpSocket;

const BIND_ANY_IPV4: &str = "0.0.0.0:0";

/// UDP sender bound to an ephemeral local port; targets a single gateway `SocketAddr`.
pub struct UdpDownlinkTransport {
    socket: UdpSocket,
    gateway: SocketAddr,
}

impl UdpDownlinkTransport {
    /// Binds `0.0.0.0:0` and remembers the gateway destination for [`RadioTransport::send_downlink`].
    pub async fn bind_ephemeral(gateway: SocketAddr) -> AppResult<Self> {
        let socket = UdpSocket::bind(BIND_ANY_IPV4)
            .await
            .map_err(|e| AppError::Infrastructure(format!("udp bind {BIND_ANY_IPV4}: {e}")))?;
        Ok(Self { socket, gateway })
    }
}

#[async_trait]
impl RadioTransport for UdpDownlinkTransport {
    async fn send_downlink(&self, frame: &DownlinkFrame) -> AppResult<()> {
        self.socket
            .send_to(&frame.payload, self.gateway)
            .await
            .map_err(|e| {
                AppError::Infrastructure(format!(
                    "udp send_to {} bytes to {}: {e}",
                    frame.payload.len(),
                    self.gateway
                ))
            })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use maverick_core::ports::RadioTransport;
    use maverick_domain::identifiers::Eui64;
    use maverick_domain::{DevAddr, GatewayEui};

    use super::UdpDownlinkTransport;
    use crate::{ResiliencePolicy, ResilientRadioTransport};

    #[tokio::test]
    async fn udp_payload_reaches_bound_listener() {
        let listener = tokio::net::UdpSocket::bind("127.0.0.1:0")
            .await
            .expect("bind listener");
        let gw = listener.local_addr().expect("local addr");

        let recv = tokio::spawn(async move {
            let mut buf = [0_u8; 64];
            let (n, _) = listener.recv_from(&mut buf).await.expect("recv");
            buf[..n].to_vec()
        });

        let sender = UdpDownlinkTransport::bind_ephemeral(gw)
            .await
            .expect("bind sender");
        let frame = maverick_core::ports::DownlinkFrame {
            gateway_eui: GatewayEui(Eui64([0_u8; 8])),
            dev_addr: DevAddr(0x01_02_03_04),
            payload: vec![0xC0, 0xFF, 0xEE],
        };
        sender.send_downlink(&frame).await.expect("send");

        let got = recv.await.expect("join");
        assert_eq!(got, frame.payload);
    }

    #[tokio::test]
    async fn resilient_wrapper_allows_success_path() {
        let listener = tokio::net::UdpSocket::bind("127.0.0.1:0")
            .await
            .expect("bind listener");
        let gw = listener.local_addr().expect("local addr");

        let _recv = tokio::spawn(async move {
            let mut buf = [0_u8; 64];
            let _ = listener.recv_from(&mut buf).await;
        });

        let udp = UdpDownlinkTransport::bind_ephemeral(gw)
            .await
            .expect("bind sender");
        let inner: Arc<dyn RadioTransport> = Arc::new(udp);
        let resilient = ResilientRadioTransport::new(inner, ResiliencePolicy::default());

        let frame = maverick_core::ports::DownlinkFrame {
            gateway_eui: GatewayEui(Eui64([0_u8; 8])),
            dev_addr: DevAddr(1),
            payload: vec![0x01],
        };
        resilient
            .send_downlink(&frame)
            .await
            .expect("resilient send");
    }
}
