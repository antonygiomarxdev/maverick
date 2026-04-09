use std::sync::Arc;

use tokio::net::UdpSocket;

use crate::db::Database;
use crate::events::{AuditRecord, EventKind, EventSource, EventStatus, SystemEvent};
use crate::kernel::KernelServices;
use crate::ports::AuditLogWriter;
use crate::use_cases::ProcessUplinkFrameCommand;
use crate::{AppError, Result};

use super::semtech::parse_push_data;

pub struct UdpIngester<D: Database> {
    services: Arc<KernelServices<D>>,
    max_datagram_size: usize,
}

impl<D: Database + Clone + Send + Sync + 'static> UdpIngester<D> {
    pub fn new(services: Arc<KernelServices<D>>, max_datagram_size: usize) -> Self {
        Self {
            services,
            max_datagram_size,
        }
    }

    pub async fn run(self) -> Result<()> {
        let socket = UdpSocket::bind(&self.services.config.udp_bind_addr)
            .await
            .map_err(AppError::Io)?;
        let mut buffer = vec![0u8; self.max_datagram_size];

        tracing::info!(
            "UDP ingester listening on {}",
            self.services.config.udp_bind_addr
        );

        loop {
            let (received, _) = socket.recv_from(&mut buffer).await.map_err(AppError::Io)?;
            let datagram = buffer[..received].to_vec();
            self.handle_datagram(datagram).await?;
        }
    }

    async fn handle_datagram(&self, datagram: Vec<u8>) -> Result<()> {
        match parse_push_data(&datagram) {
            Ok(parsed) => {
                let service = self.services.ingest_uplink_service();
                let frame_service = self.services.process_uplink_frame_service();

                for command in parsed.commands {
                    let frame_cmd = ProcessUplinkFrameCommand {
                        uplink: command.uplink.clone(),
                        correlation_id: command.correlation_id.clone(),
                    };
                    service.ingest(command).await?;
                    match frame_service.process(frame_cmd).await {
                        Ok(outcome) => {
                            tracing::info!(outcome = ?outcome, "lorawan frame processed")
                        }
                        Err(e) => {
                            tracing::warn!(error = %e, "lorawan frame processing failed (non-fatal)")
                        }
                    }
                }
            }
            Err(error) => {
                let summary = error.to_string();
                let audit = AuditRecord::new(
                    EventSource::Udp,
                    "udp.push_data.rejected",
                    "udp_datagram",
                    EventStatus::Rejected,
                    &summary,
                )
                .with_metadata("payload_size", datagram.len().to_string());
                self.services.audit_log.record(audit).await?;

                self.services.event_bus.publish(
                    SystemEvent::new(
                        EventKind::UplinkObservation,
                        EventSource::Udp,
                        "udp.push_data.rejected",
                        EventStatus::Rejected,
                    )
                    .with_reason_code("semtech_parse_failed")
                    .with_metadata("payload_size", datagram.len().to_string())
                    .with_metadata("summary", summary),
                );
            }
        }

        Ok(())
    }
}

pub fn run_udp_ingester<D: Database + Clone + Send + Sync + 'static>(
    services: Arc<KernelServices<D>>,
    max_datagram_size: usize,
) -> tokio::task::JoinHandle<Result<()>> {
    tokio::spawn(async move { UdpIngester::new(services, max_datagram_size).run().await })
}
