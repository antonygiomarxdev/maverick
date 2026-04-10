use std::sync::Arc;

use crate::error::{AppError, AppResult};
use crate::ports::{
    AuditRecord, AuditSink, SessionRepository, UplinkObservation, UplinkRecord, UplinkRepository,
};
use crate::protocol::{ProtocolCapability, ProtocolContext};

/// Application service: validate uplink via protocol module, persist, audit.
pub struct IngestUplink {
    pub sessions: Arc<dyn SessionRepository>,
    pub uplinks: Arc<dyn UplinkRepository>,
    pub audit: Arc<dyn AuditSink>,
    pub protocol: Arc<dyn ProtocolCapability>,
}

impl IngestUplink {
    pub async fn execute(&self, obs: UplinkObservation) -> AppResult<()> {
        let session = self.sessions.get_by_dev_addr(obs.dev_addr).await?;

        let ctx = ProtocolContext {
            observation: &obs,
            session: session.as_ref(),
        };
        let decision = self.protocol.validate_uplink(ctx)?;
        match decision {
            crate::protocol::ProtocolDecision::Accept => {}
            other => {
                self.audit
                    .emit(AuditRecord {
                        source: "kernel".to_string(),
                        operation: "ingest_uplink".to_string(),
                        entity_type: "uplink".to_string(),
                        entity_id: Some(format!("{:08x}", obs.dev_addr.0)),
                        outcome: format!("rejected:{other:?}"),
                        metadata: None,
                    })
                    .await?;
                return Err(AppError::Domain(format!("uplink rejected: {other:?}")));
            }
        }

        let Some(session) = session else {
            return Err(AppError::NotFound("session".to_string()));
        };

        self.uplinks
            .append(&UplinkRecord {
                dev_addr: obs.dev_addr,
                f_cnt: obs.f_cnt,
                payload: obs.payload.clone(),
            })
            .await?;

        let mut updated = session;
        updated.uplink_frame_counter = obs.f_cnt;
        self.sessions.upsert(&updated).await?;

        self.audit
            .emit(AuditRecord {
                source: "kernel".to_string(),
                operation: "ingest_uplink".to_string(),
                entity_type: "uplink".to_string(),
                entity_id: Some(format!("{:08x}", obs.dev_addr.0)),
                outcome: "success".to_string(),
                metadata: None,
            })
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use maverick_domain::identifiers::Eui64;
    use maverick_domain::{DevAddr, DevEui, DeviceClass, GatewayEui, RegionId, SessionSnapshot};

    use super::*;
    use crate::ports::{SessionRepository, UplinkRepository};
    use crate::protocol::LoRaWAN10xClassA;

    struct MemSession(Arc<tokio::sync::Mutex<Option<SessionSnapshot>>>);
    struct MemUplinks(Arc<tokio::sync::Mutex<Vec<UplinkRecord>>>);
    struct MemAudit(Arc<tokio::sync::Mutex<Vec<String>>>);

    #[async_trait]
    impl SessionRepository for MemSession {
        async fn get_by_dev_addr(&self, dev_addr: DevAddr) -> AppResult<Option<SessionSnapshot>> {
            let g = self.0.lock().await;
            Ok(g.as_ref().filter(|s| s.dev_addr == dev_addr).cloned())
        }

        async fn upsert(&self, session: &SessionSnapshot) -> AppResult<()> {
            *self.0.lock().await = Some(session.clone());
            Ok(())
        }
    }

    #[async_trait]
    impl UplinkRepository for MemUplinks {
        async fn append(&self, record: &UplinkRecord) -> AppResult<()> {
            self.0.lock().await.push(record.clone());
            Ok(())
        }
    }

    #[async_trait]
    impl AuditSink for MemAudit {
        async fn emit(&self, record: AuditRecord) -> AppResult<()> {
            self.0.lock().await.push(record.outcome);
            Ok(())
        }
    }

    fn obs(fc: u32) -> UplinkObservation {
        UplinkObservation {
            gateway_eui: GatewayEui(Eui64([9; 8])),
            dev_addr: DevAddr(0xAB_CD_00_01),
            region: RegionId::Eu868,
            f_cnt: fc,
            f_port: 1,
            payload: vec![0xAA],
            rssi: Some(-90),
            snr: Some(5.5),
        }
    }

    #[tokio::test]
    async fn ingest_happy_path_updates_session_and_uplink() {
        let session = SessionSnapshot {
            dev_eui: DevEui(Eui64([1; 8])),
            dev_addr: DevAddr(0xAB_CD_00_01),
            region: RegionId::Eu868,
            class: DeviceClass::ClassA,
            uplink_frame_counter: 0,
            downlink_frame_counter: 0,
        };
        let sess_store = Arc::new(tokio::sync::Mutex::new(Some(session)));
        let uplinks = Arc::new(tokio::sync::Mutex::new(Vec::new()));
        let audit = Arc::new(tokio::sync::Mutex::new(Vec::new()));

        let svc = IngestUplink {
            sessions: Arc::new(MemSession(sess_store.clone())),
            uplinks: Arc::new(MemUplinks(uplinks.clone())),
            audit: Arc::new(MemAudit(audit.clone())),
            protocol: Arc::new(LoRaWAN10xClassA),
        };

        svc.execute(obs(1)).await.expect("ingest");
        assert_eq!(uplinks.lock().await.len(), 1);
        assert_eq!(
            sess_store
                .lock()
                .await
                .as_ref()
                .unwrap()
                .uplink_frame_counter,
            1
        );
    }

    #[tokio::test]
    async fn ingest_rejects_bad_fcnt() {
        let session = SessionSnapshot {
            dev_eui: DevEui(Eui64([1; 8])),
            dev_addr: DevAddr(0xAB_CD_00_01),
            region: RegionId::Eu868,
            class: DeviceClass::ClassA,
            uplink_frame_counter: 5,
            downlink_frame_counter: 0,
        };
        let sess_store = Arc::new(tokio::sync::Mutex::new(Some(session)));
        let uplinks = Arc::new(tokio::sync::Mutex::new(Vec::new()));
        let audit = Arc::new(tokio::sync::Mutex::new(Vec::new()));

        let svc = IngestUplink {
            sessions: Arc::new(MemSession(sess_store)),
            uplinks: Arc::new(MemUplinks(uplinks.clone())),
            audit: Arc::new(MemAudit(audit.clone())),
            protocol: Arc::new(LoRaWAN10xClassA),
        };

        let err = svc.execute(obs(5)).await.unwrap_err();
        assert!(matches!(err, AppError::Domain(_)));
        assert!(uplinks.lock().await.is_empty());
    }
}
