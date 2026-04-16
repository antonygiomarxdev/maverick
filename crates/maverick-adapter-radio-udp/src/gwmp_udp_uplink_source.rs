//! GWMP-over-UDP [`UplinkSource`](maverick_core::ports::UplinkSource): one datagram per `next_batch` poll.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use maverick_core::error::{AppError, AppResult};
use maverick_core::ports::{UplinkReceive, UplinkSource};

use crate::gwmp::parse_push_data;

/// Matches `gwmp_loop` historical recv buffer size.
const GWMP_UDP_RECV_BUFFER_LEN: usize = 4096;

/// Semtech `PUSH_DATA` UDP source: blocking-style `next_batch` with read timeout → idle or observations.
#[derive(Debug)]
pub struct GwmpUdpUplinkSource {
    socket: Arc<tokio::net::UdpSocket>,
    read_timeout: Duration,
    recv_buffer_len: usize,
}

impl GwmpUdpUplinkSource {
    /// Construct from an already-bound socket (composition root may bind earlier for diagnostics).
    pub fn new(socket: Arc<tokio::net::UdpSocket>, read_timeout: Duration, recv_buffer_len: usize) -> Self {
        Self {
            socket,
            read_timeout,
            recv_buffer_len: recv_buffer_len.max(512),
        }
    }

    /// Bind UDP and use the default GWMP recv buffer size.
    pub async fn bind(
        bind_addr: impl tokio::net::ToSocketAddrs,
        read_timeout: Duration,
    ) -> AppResult<Self> {
        let socket = tokio::net::UdpSocket::bind(bind_addr)
            .await
            .map_err(|e| AppError::Infrastructure(format!("udp bind: {e}")))?;
        Ok(Self::new(
            Arc::new(socket),
            read_timeout,
            GWMP_UDP_RECV_BUFFER_LEN,
        ))
    }
}

#[async_trait]
impl UplinkSource for GwmpUdpUplinkSource {
    async fn next_batch(&self) -> AppResult<UplinkReceive> {
        let mut buf = vec![0_u8; self.recv_buffer_len];
        let recv = tokio::time::timeout(self.read_timeout, self.socket.recv_from(&mut buf)).await;
        match recv {
            Err(_) => Ok(UplinkReceive::Idle),
            Ok(Err(e)) => Err(AppError::Infrastructure(format!("udp recv: {e}"))),
            Ok(Ok((n, _addr))) => {
                let batch = parse_push_data(&buf[..n])?;
                Ok(UplinkReceive::Observations(batch.observations))
            }
        }
    }
}
