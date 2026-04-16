use std::sync::Arc;

use aes::Aes128;
use cmac::{Cmac, KeyInit, Mac};

use crate::error::{AppError, AppResult};
use crate::ports::{
    AuditRecord, AuditSink, SessionRepository, UplinkObservation, UplinkRecord, UplinkRepository,
};
use crate::protocol::{FcntError, LoRaWAN10xClassA, ProtocolCapability, ProtocolContext};

/// Application service: validate uplink via protocol module, persist, audit.
pub struct IngestUplink {
    pub sessions: Arc<dyn SessionRepository>,
    pub uplinks: Arc<dyn UplinkRepository>,
    pub audit: Arc<dyn AuditSink>,
    pub protocol: Arc<dyn ProtocolCapability>,
}

/// LoRaWAN 1.0.x §4.4 — B0 block for uplink MIC.
/// All multi-byte fields are LITTLE-ENDIAN per spec.
pub fn build_b0_uplink(dev_addr: u32, f_cnt: u32, phy_len_without_mic: usize) -> [u8; 16] {
    let mut b0 = [0u8; 16];
    b0[0] = 0x49;
    b0[5] = 0x00; // uplink direction
    b0[6..10].copy_from_slice(&dev_addr.to_le_bytes()); // PITFALL: must be LE
    b0[10..14].copy_from_slice(&f_cnt.to_le_bytes()); // PITFALL: must be LE
    b0[15] = phy_len_without_mic as u8;
    b0
}

/// Compute AES-128 CMAC over B0 || PHY_without_MIC, return first 4 bytes.
pub fn compute_mic(
    nwk_s_key: &[u8; 16],
    b0: &[u8; 16],
    phy_without_mic: &[u8],
) -> [u8; 4] {
    let mut mac =
        <Cmac<Aes128> as KeyInit>::new_from_slice(nwk_s_key).expect("NwkSKey is always 16 bytes");
    mac.update(b0);
    mac.update(phy_without_mic);
    let full = mac.finalize().into_bytes();
    [full[0], full[1], full[2], full[3]]
}

/// LoRaWAN 1.0.x §4.3.3.2 — AES-128-CTR FRMPayload decryption.
/// Block counter `i` starts at 1 (NOT 0) per spec §4.3.3.2.
fn decrypt_frm_payload(
    app_s_key: &[u8; 16],
    dev_addr: u32,
    f_cnt: u32,
    payload: &[u8],
) -> Vec<u8> {
    use aes::cipher::BlockCipherEncrypt;
    if payload.is_empty() {
        return Vec::new();
    }
    let cipher =
        <Aes128 as KeyInit>::new_from_slice(app_s_key).expect("AppSKey is always 16 bytes");
    let block_count = payload.len().div_ceil(16);
    let mut keystream = Vec::with_capacity(block_count * 16);
    for i in 1u8..=(block_count as u8) {
        // CRITICAL: counter starts at 1, not 0 (LoRaWAN §4.3.3.2)
        let mut ai = [0u8; 16];
        ai[0] = 0x01;
        ai[5] = 0x00; // uplink direction
        ai[6..10].copy_from_slice(&dev_addr.to_le_bytes()); // PITFALL: must be LE
        ai[10..14].copy_from_slice(&f_cnt.to_le_bytes()); // PITFALL: must be LE
        ai[15] = i;
        let mut block = aes::Block::from(ai);
        cipher.encrypt_block(&mut block);
        keystream.extend_from_slice(&block);
    }
    payload
        .iter()
        .zip(keystream.iter())
        .map(|(p, k)| p ^ k)
        .collect()
}

fn now_ms_portable() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis().min(i64::MAX as u128) as i64)
        .unwrap_or(0)
}

impl IngestUplink {
    pub async fn execute(&self, obs: UplinkObservation) -> AppResult<()> {
        // 1. Fetch session (includes keys)
        let session = self.sessions.get_by_dev_addr(obs.dev_addr).await?;

        // 2. No-session fast path — skip FCnt/MIC, let protocol report it
        let Some(session_ref) = session.as_ref() else {
            let ctx = ProtocolContext {
                observation: &obs,
                session: None,
            };
            let decision = self.protocol.validate_uplink(ctx)?;
            self.audit
                .emit(AuditRecord {
                    source: "kernel".to_string(),
                    operation: "ingest_uplink".to_string(),
                    entity_type: "uplink".to_string(),
                    entity_id: Some(format!("{:08x}", obs.dev_addr.0)),
                    outcome: format!("rejected:{decision:?}"),
                    metadata: None,
                })
                .await?;
            return Err(AppError::Domain(format!("uplink rejected: {decision:?}")));
        };

        // 3. FCnt 32-bit reconstruction (LoRaWAN §4.3.1.5) — before validate_uplink
        let reconstructed_fcnt =
            match LoRaWAN10xClassA::extend_fcnt(obs.f_cnt, session_ref.uplink_frame_counter) {
                Ok(fc) => fc,
                Err(FcntError::Duplicate) => {
                    self.audit
                        .emit(AuditRecord {
                            source: "kernel".to_string(),
                            operation: "ingest_uplink".to_string(),
                            entity_type: "uplink".to_string(),
                            entity_id: Some(format!("{:08x}", obs.dev_addr.0)),
                            outcome: "rejected:RejectDuplicateFrameCounter".to_string(),
                            metadata: None,
                        })
                        .await?;
                    return Err(AppError::Domain(
                        "uplink rejected: RejectDuplicateFrameCounter".to_string(),
                    ));
                }
                Err(FcntError::GapExceeded) => {
                    self.audit
                        .emit(AuditRecord {
                            source: "kernel".to_string(),
                            operation: "ingest_uplink".to_string(),
                            entity_type: "uplink".to_string(),
                            entity_id: Some(format!("{:08x}", obs.dev_addr.0)),
                            outcome: "rejected:RejectFcntGapExceeded".to_string(),
                            metadata: None,
                        })
                        .await?;
                    return Err(AppError::Domain(
                        "uplink rejected: RejectFcntGapExceeded".to_string(),
                    ));
                }
            };

        // 4. Protocol validation (region, class)
        let ctx = ProtocolContext {
            observation: &obs,
            session: Some(session_ref),
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

        let session = session.expect("session confirmed present before this point");

        // 5. MIC verification (LoRaWAN §4.4) — using reconstructed 32-bit FCnt
        let b0 = build_b0_uplink(obs.dev_addr.0, reconstructed_fcnt, obs.phy_without_mic.len());
        let computed_mic = compute_mic(&session.nwk_s_key, &b0, &obs.phy_without_mic);
        if computed_mic != obs.wire_mic {
            self.audit
                .emit(AuditRecord {
                    source: "kernel".to_string(),
                    operation: "ingest_uplink".to_string(),
                    entity_type: "uplink".to_string(),
                    entity_id: Some(format!("{:08x}", obs.dev_addr.0)),
                    outcome: "rejected:mic_invalid".to_string(),
                    metadata: None,
                })
                .await?;
            return Err(AppError::Domain("mic_invalid".to_string()));
        }

        // 6. Payload decryption (LoRaWAN §4.3.3.2) — after MIC passes
        let payload_decrypted = if obs.payload.is_empty() {
            None
        } else {
            Some(decrypt_frm_payload(
                &session.app_s_key,
                obs.dev_addr.0,
                reconstructed_fcnt,
                &obs.payload,
            ))
        };

        // 7. Persist uplink
        self.uplinks
            .append(&UplinkRecord {
                dev_addr: obs.dev_addr,
                f_cnt: reconstructed_fcnt,
                received_at_ms: now_ms_portable(),
                payload: obs.payload.clone(),
                application_id: session.application_id.clone(),
                payload_decrypted,
            })
            .await?;

        // 8. Update session counter to reconstructed 32-bit value
        let mut updated = session;
        updated.uplink_frame_counter = reconstructed_fcnt;
        self.sessions.upsert(&updated).await?;

        // 9. Audit success
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

    fn sample_session() -> SessionSnapshot {
        SessionSnapshot {
            dev_eui: DevEui(Eui64([1; 8])),
            dev_addr: DevAddr(0xAB_CD_00_01),
            region: RegionId::Eu868,
            class: DeviceClass::ClassA,
            uplink_frame_counter: 0,
            downlink_frame_counter: 0,
            application_id: None,
            nwk_s_key: [0u8; 16],
            app_s_key: [0u8; 16],
        }
    }

    /// Build an observation with a valid MIC for the zero NwkSKey.
    fn obs_with_valid_mic(fc: u16, session: &SessionSnapshot) -> UplinkObservation {
        let phy: Vec<u8> = vec![0xAA]; // dummy PHY body (no actual LoRaWAN framing needed for unit test)
        // Compute what reconstructed FCnt will be (wire > session → candidate_low)
        let reconstructed = u32::from(fc); // session.uplink_frame_counter = 0, so candidate_low = fc
        let b0 = build_b0_uplink(session.dev_addr.0, reconstructed, phy.len());
        let mic = compute_mic(&session.nwk_s_key, &b0, &phy);
        UplinkObservation {
            gateway_eui: GatewayEui(Eui64([9; 8])),
            dev_addr: session.dev_addr,
            region: RegionId::Eu868,
            f_cnt: fc,
            f_port: 1,
            payload: vec![0xAA],
            rssi: Some(-90),
            snr: Some(5.5),
            wire_mic: mic,
            phy_without_mic: phy,
        }
    }

    /// Build an observation with a zero MIC (for tests that expect FCnt rejection before MIC check).
    fn obs_with_zero_mic(fc: u16) -> UplinkObservation {
        UplinkObservation {
            gateway_eui: GatewayEui(Eui64([9; 8])),
            dev_addr: DevAddr(0xAB_CD_00_01),
            region: RegionId::Eu868,
            f_cnt: fc,
            f_port: 1,
            payload: vec![0xAA],
            rssi: Some(-90),
            snr: Some(5.5),
            wire_mic: [0u8; 4],
            phy_without_mic: vec![],
        }
    }

    #[tokio::test]
    async fn ingest_happy_path_updates_session_and_uplink() {
        let session = sample_session();
        let sess_store = Arc::new(tokio::sync::Mutex::new(Some(session.clone())));
        let uplinks = Arc::new(tokio::sync::Mutex::new(Vec::new()));
        let audit = Arc::new(tokio::sync::Mutex::new(Vec::new()));

        let svc = IngestUplink {
            sessions: Arc::new(MemSession(sess_store.clone())),
            uplinks: Arc::new(MemUplinks(uplinks.clone())),
            audit: Arc::new(MemAudit(audit.clone())),
            protocol: Arc::new(LoRaWAN10xClassA),
        };

        let obs = obs_with_valid_mic(1, &session);
        svc.execute(obs).await.expect("ingest");
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
        let mut session = sample_session();
        session.uplink_frame_counter = 5;
        let sess_store = Arc::new(tokio::sync::Mutex::new(Some(session)));
        let uplinks = Arc::new(tokio::sync::Mutex::new(Vec::new()));
        let audit = Arc::new(tokio::sync::Mutex::new(Vec::new()));

        let svc = IngestUplink {
            sessions: Arc::new(MemSession(sess_store)),
            uplinks: Arc::new(MemUplinks(uplinks.clone())),
            audit: Arc::new(MemAudit(audit.clone())),
            protocol: Arc::new(LoRaWAN10xClassA),
        };

        // wire fc=5 with session=5 → Duplicate (rejected before MIC check)
        let err = svc.execute(obs_with_zero_mic(5)).await.unwrap_err();
        assert!(matches!(err, AppError::Domain(_)));
        assert!(uplinks.lock().await.is_empty());
    }

    #[tokio::test]
    async fn ingest_rejects_bad_mic() {
        let session = sample_session();
        let sess_store = Arc::new(tokio::sync::Mutex::new(Some(session.clone())));
        let uplinks = Arc::new(tokio::sync::Mutex::new(Vec::new()));
        let audit = Arc::new(tokio::sync::Mutex::new(Vec::new()));

        let svc = IngestUplink {
            sessions: Arc::new(MemSession(sess_store)),
            uplinks: Arc::new(MemUplinks(uplinks.clone())),
            audit: Arc::new(MemAudit(audit.clone())),
            protocol: Arc::new(LoRaWAN10xClassA),
        };

        let mut wrong_obs = obs_with_valid_mic(1, &session);
        wrong_obs.wire_mic = [0xFF, 0xFF, 0xFF, 0xFF]; // deliberately wrong
        let err = svc.execute(wrong_obs).await.unwrap_err();
        assert!(matches!(err, AppError::Domain(ref s) if s.contains("mic_invalid")));
        assert!(uplinks.lock().await.is_empty());
    }
}
