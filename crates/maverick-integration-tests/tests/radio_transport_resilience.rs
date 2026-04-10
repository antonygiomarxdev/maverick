//! Cross-crate composition: resilient UDP downlink (Slice 3 transport boundary).

use std::sync::Arc;

use maverick_adapter_radio_udp::{
    parse_push_data, CircuitStateView, ResiliencePolicy, ResilientRadioTransport,
    UdpDownlinkTransport, UdpRadioStub,
};
use maverick_core::error::{AppError, AppResult};
use maverick_core::ports::{DownlinkFrame, RadioTransport};
use maverick_domain::identifiers::Eui64;
use maverick_domain::{DevAddr, GatewayEui};

fn sample_frame() -> DownlinkFrame {
    DownlinkFrame {
        gateway_eui: GatewayEui(Eui64([0_u8; 8])),
        dev_addr: DevAddr(0x10_20_30_40),
        payload: vec![0xDE, 0xAD],
    }
}

#[tokio::test]
async fn integration_udp_downlink_through_resilient_wrapper() {
    let listener = tokio::net::UdpSocket::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let gw = listener.local_addr().expect("listener addr");

    let recv = tokio::spawn(async move {
        let mut buf = [0_u8; 64];
        let (n, _) = listener.recv_from(&mut buf).await.expect("recv");
        buf[..n].to_vec()
    });

    let udp = UdpDownlinkTransport::bind_ephemeral(gw)
        .await
        .expect("bind sender");
    let inner: Arc<dyn RadioTransport> = Arc::new(udp);
    let transport = ResilientRadioTransport::new(inner, ResiliencePolicy::default());

    let frame = sample_frame();
    transport.send_downlink(&frame).await.expect("send");

    let got = recv.await.expect("join");
    assert_eq!(got, frame.payload);
}

#[tokio::test]
async fn stub_adapter_fails_without_panicking_kernel_contract() {
    let stub = UdpRadioStub;
    let err = stub
        .send_downlink(&sample_frame())
        .await
        .expect_err("stub must error");
    assert!(matches!(err, AppError::Infrastructure(_)));
}

struct FailThenSucceed {
    state: tokio::sync::Mutex<u8>,
}

#[async_trait::async_trait]
impl RadioTransport for FailThenSucceed {
    async fn send_downlink(&self, _frame: &DownlinkFrame) -> AppResult<()> {
        let mut g = self.state.lock().await;
        if *g < 1 {
            *g += 1;
            Err(AppError::Infrastructure("transient".to_string()))
        } else {
            Ok(())
        }
    }
}

#[tokio::test]
async fn circuit_recovers_after_open_window_and_successful_trial() {
    let policy = ResiliencePolicy {
        max_retries: 0,
        circuit_failure_threshold: 1,
        circuit_open_duration: std::time::Duration::from_millis(30),
        ..ResiliencePolicy::default()
    };
    let inner: Arc<dyn RadioTransport> = Arc::new(FailThenSucceed {
        state: tokio::sync::Mutex::new(0),
    });
    let transport = ResilientRadioTransport::new(inner, policy);
    let frame = sample_frame();

    let _ = transport.send_downlink(&frame).await;
    let err = transport
        .send_downlink(&frame)
        .await
        .expect_err("expected open circuit");
    assert!(matches!(err, AppError::CircuitOpen(_)));
    tokio::time::sleep(std::time::Duration::from_millis(40)).await;
    let _ = transport.send_downlink(&frame).await;
    assert_eq!(transport.circuit_state(), CircuitStateView::Closed);
}

#[tokio::test]
async fn parse_failure_is_reported_without_panic() {
    let malformed = b"not-gwmp";
    let err = parse_push_data(malformed).expect_err("expected parse failure");
    assert!(matches!(err, AppError::InvalidInput(_)));
}
