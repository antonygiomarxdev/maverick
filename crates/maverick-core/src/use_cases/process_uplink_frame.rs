use maverick_crypto::JoinCrypto;
use maverick_domain::{DeviceSession, Eui64, UplinkFrame};

use crate::events::{AuditRecord, EventBus, EventKind, EventSource, EventStatus, SystemEvent};
use crate::ports::{AuditLogWriter, DeviceRepository, SessionRepository};
use crate::{AppError, DomainError, Result};

#[derive(Debug, Clone)]
pub struct ProcessUplinkFrameCommand {
    pub uplink: UplinkFrame,
    pub correlation_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessUplinkFrameOutcome {
    JoinRequestObserved {
        dev_eui: Eui64,
        app_eui: Eui64,
        dev_nonce: u16,
    },
    DataUplinkReady {
        dev_eui: Eui64,
        dev_addr: u32,
        frame_counter: u32,
        confirmed: bool,
    },
    Unsupported {
        mtype: u8,
    },
    JoinAccepted {
        dev_eui: Eui64,
        dev_addr: u32,
    },
    Rejected {
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedLorawanFrame {
    JoinRequest {
        app_eui: Eui64,
        dev_eui: Eui64,
        dev_nonce: u16,
        mic: [u8; 4],
    },
    DataUplink {
        confirmed: bool,
        dev_addr: u32,
        fctrl: u8,
        frame_counter: u32,
        fport: Option<u8>,
        frm_payload: Vec<u8>,
        mic: [u8; 4],
    },
    Unsupported {
        mtype: u8,
    },
}

pub struct ProcessUplinkFrameService<S, A, R> {
    sessions: S,
    audit_log: A,
    event_bus: EventBus,
    devices: R,
}

impl<S, A, R> ProcessUplinkFrameService<S, A, R>
where
    S: SessionRepository,
    A: AuditLogWriter,
    R: DeviceRepository,
{
    pub fn new(sessions: S, audit_log: A, event_bus: EventBus, devices: R) -> Self {
        Self {
            sessions,
            audit_log,
            event_bus,
            devices,
        }
    }

    pub async fn process(
        &self,
        command: ProcessUplinkFrameCommand,
    ) -> Result<ProcessUplinkFrameOutcome> {
        let parsed = parse_lorawan_frame(&command.uplink.payload)?;

        match parsed {
            ParsedLorawanFrame::JoinRequest {
                app_eui,
                dev_eui,
                dev_nonce,
                ..
            } => {
                // Look up device by dev_eui
                let Some(device) = self.devices.get_by_dev_eui(dev_eui).await? else {
                    let audit = AuditRecord::new(
                        EventSource::Udp,
                        "lorawan.join_request.unknown_device",
                        "uplink_frame",
                        EventStatus::Rejected,
                        "join request from unregistered device",
                    )
                    .with_metadata("dev_eui", dev_eui.to_string());
                    self.audit_log.record(audit).await?;
                    self.event_bus.publish(
                        SystemEvent::new(
                            EventKind::UplinkObservation,
                            EventSource::Udp,
                            "lorawan.join_request.unknown_device",
                            EventStatus::Rejected,
                        )
                        .with_entity_id(dev_eui.to_string())
                        .with_reason_code("device_not_found"),
                    );
                    return Ok(ProcessUplinkFrameOutcome::Rejected {
                        reason: format!("unknown_device:{dev_eui}"),
                    });
                };

                // Validate JoinRequest MIC using AppKey
                let app_key = device.keys.app_key.as_bytes();
                match JoinCrypto::validate_join_request_mic(&app_key, &command.uplink.payload) {
                    Ok(false) => {
                        let audit = AuditRecord::new(
                            EventSource::Udp,
                            "lorawan.join_request.mic.rejected",
                            "uplink_frame",
                            EventStatus::Rejected,
                            "join request MIC validation failed",
                        )
                        .with_metadata("dev_eui", dev_eui.to_string());
                        self.audit_log.record(audit).await?;
                        return Ok(ProcessUplinkFrameOutcome::Rejected {
                            reason: "join_mic_invalid".to_string(),
                        });
                    }
                    Err(e) => tracing::warn!(error = %e, "join MIC computation error, proceeding"),
                    Ok(true) => {}
                }

                // Derive session keys per LoRaWAN 1.0.3 §6.2.5
                let now_ns = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .subsec_nanos();
                let app_nonce = [(now_ns >> 16) as u8, (now_ns >> 8) as u8, now_ns as u8];
                let net_id = [0x00u8, 0x00, 0x00];
                let (nwk_s_key, app_s_key) =
                    JoinCrypto::derive_session_keys(&app_key, app_nonce, net_id, dev_nonce)
                        .map_err(|e| AppError::Database(e.to_string()))?;

                let dev_addr = JoinCrypto::derive_dev_addr(&app_key, dev_eui.as_bytes_slice())
                    .map_err(|e| AppError::Database(e.to_string()))?;

                let session = DeviceSession::new(dev_addr, app_s_key, nwk_s_key);
                self.sessions.upsert_for_device(dev_eui, session).await?;

                self.audit_log
                    .record(join_observed_audit(
                        &dev_eui,
                        command.correlation_id.clone(),
                        command.uplink.payload.len(),
                    ))
                    .await?;
                self.event_bus.publish(
                    SystemEvent::new(
                        EventKind::UplinkObservation,
                        EventSource::Udp,
                        "lorawan.join_request.accepted",
                        EventStatus::Accepted,
                    )
                    .with_entity_id(dev_eui.to_string())
                    .with_metadata("app_eui", app_eui.to_string())
                    .with_metadata("dev_nonce", dev_nonce.to_string())
                    .with_metadata("dev_addr", format!("{dev_addr:08X}")),
                );

                Ok(ProcessUplinkFrameOutcome::JoinAccepted { dev_eui, dev_addr })
            }
            ParsedLorawanFrame::DataUplink {
                confirmed,
                dev_addr,
                frame_counter,
                mic,
                ..
            } => {
                let Some((dev_eui, session)) = self.sessions.get_by_dev_addr(dev_addr).await?
                else {
                    return Err(DomainError::NotFound {
                        entity: "device_session",
                        id: format!("dev_addr:{dev_addr:08X}"),
                    }
                    .into());
                };

                if frame_counter < session.frame_counter.0 {
                    return Err(DomainError::InvalidState {
                        entity: "uplink",
                        reason: format!(
                            "stale frame counter {frame_counter} for dev_addr {dev_addr:08X}; current {}",
                            session.frame_counter.0
                        ),
                    }
                    .into());
                }

                // MIC validation per LoRaWAN 1.0.3 §4.4
                let payload_without_mic =
                    &command.uplink.payload[..command.uplink.payload.len().saturating_sub(4)];
                match JoinCrypto::validate_uplink_mic(
                    &session.nwk_s_key,
                    dev_addr,
                    frame_counter,
                    payload_without_mic,
                    &mic,
                ) {
                    Ok(false) => {
                        let mic_audit = AuditRecord::new(
                            EventSource::Udp,
                            "lorawan.uplink.mic.rejected",
                            "uplink_frame",
                            EventStatus::Rejected,
                            "MIC validation failed",
                        )
                        .with_metadata("dev_addr", format!("{dev_addr:08X}"))
                        .with_metadata("frame_counter", frame_counter.to_string());
                        self.audit_log.record(mic_audit).await?;
                        self.event_bus.publish(
                            SystemEvent::new(
                                EventKind::UplinkObservation,
                                EventSource::Udp,
                                "lorawan.uplink.mic.rejected",
                                EventStatus::Rejected,
                            )
                            .with_entity_id(dev_eui.to_string())
                            .with_reason_code("mic_invalid"),
                        );
                        return Ok(ProcessUplinkFrameOutcome::Rejected {
                            reason: "mic_invalid".to_string(),
                        });
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "uplink MIC computation error, proceeding")
                    }
                    Ok(true) => {}
                }

                let mut audit = AuditRecord::new(
                    EventSource::Udp,
                    "lorawan.uplink.validated",
                    "uplink_frame",
                    EventStatus::Accepted,
                    "lorawan uplink validated",
                )
                .with_metadata("dev_addr", format!("{dev_addr:08X}"))
                .with_metadata("frame_counter", frame_counter.to_string())
                .with_metadata("confirmed", confirmed.to_string());
                audit.entity_id = Some(dev_eui.to_string());
                audit.correlation_id = command.correlation_id.clone();
                self.audit_log.record(audit).await?;

                self.event_bus.publish(
                    SystemEvent::new(
                        EventKind::UplinkObservation,
                        EventSource::Udp,
                        "lorawan.uplink.validated",
                        EventStatus::Accepted,
                    )
                    .with_entity_id(dev_eui.to_string())
                    .with_metadata("dev_addr", format!("{dev_addr:08X}"))
                    .with_metadata("frame_counter", frame_counter.to_string())
                    .with_metadata("confirmed", confirmed.to_string()),
                );

                Ok(ProcessUplinkFrameOutcome::DataUplinkReady {
                    dev_eui,
                    dev_addr,
                    frame_counter,
                    confirmed,
                })
            }
            ParsedLorawanFrame::Unsupported { mtype } => {
                self.audit_log
                    .record(
                        AuditRecord::new(
                            EventSource::Udp,
                            "lorawan.frame.unsupported",
                            "uplink_frame",
                            EventStatus::Rejected,
                            "lorawan frame type not supported in current block",
                        )
                        .with_metadata("mtype", mtype.to_string()),
                    )
                    .await?;

                self.event_bus.publish(
                    SystemEvent::new(
                        EventKind::UplinkObservation,
                        EventSource::Udp,
                        "lorawan.frame.unsupported",
                        EventStatus::Rejected,
                    )
                    .with_reason_code("unsupported_mtype")
                    .with_metadata("mtype", mtype.to_string()),
                );

                Ok(ProcessUplinkFrameOutcome::Unsupported { mtype })
            }
        }
    }
}

pub fn parse_lorawan_frame(payload: &[u8]) -> Result<ParsedLorawanFrame> {
    if payload.len() < 5 {
        return Err(DomainError::Validation {
            field: "lorawan_payload",
            reason: "frame too short to include MHDR and MIC".to_string(),
        }
        .into());
    }

    let mhdr = payload[0];
    let mtype = (mhdr >> 5) & 0x07;

    match mtype {
        0 => parse_join_request(payload),
        2 => parse_data_uplink(payload, false),
        4 => parse_data_uplink(payload, true),
        _ => Ok(ParsedLorawanFrame::Unsupported { mtype }),
    }
}

fn parse_join_request(payload: &[u8]) -> Result<ParsedLorawanFrame> {
    if payload.len() != 23 {
        return Err(DomainError::Validation {
            field: "lorawan_payload",
            reason: format!("join request must be 23 bytes, got {}", payload.len()),
        }
        .into());
    }

    let app_eui_bytes: [u8; 8] = payload[1..9]
        .try_into()
        .map_err(|_| DomainError::Validation {
            field: "join_request.app_eui",
            reason: "invalid app_eui bytes".to_string(),
        })?;
    let app_eui = Eui64::from(app_eui_bytes);

    let dev_eui_bytes: [u8; 8] =
        payload[9..17]
            .try_into()
            .map_err(|_| DomainError::Validation {
                field: "join_request.dev_eui",
                reason: "invalid dev_eui bytes".to_string(),
            })?;
    let dev_eui = Eui64::from(dev_eui_bytes);
    let dev_nonce = u16::from_le_bytes([payload[17], payload[18]]);
    let mic = [payload[19], payload[20], payload[21], payload[22]];

    Ok(ParsedLorawanFrame::JoinRequest {
        app_eui,
        dev_eui,
        dev_nonce,
        mic,
    })
}

fn parse_data_uplink(payload: &[u8], confirmed: bool) -> Result<ParsedLorawanFrame> {
    if payload.len() < 12 {
        return Err(DomainError::Validation {
            field: "lorawan_payload",
            reason: format!("data uplink too short: {}", payload.len()),
        }
        .into());
    }

    let mic_start = payload.len() - 4;
    let mic = [
        payload[mic_start],
        payload[mic_start + 1],
        payload[mic_start + 2],
        payload[mic_start + 3],
    ];
    let mac_payload = &payload[1..mic_start];

    if mac_payload.len() < 7 {
        return Err(DomainError::Validation {
            field: "lorawan_payload.fhdr",
            reason: "fhdr too short".to_string(),
        }
        .into());
    }

    let dev_addr =
        u32::from_le_bytes(
            mac_payload[0..4]
                .try_into()
                .map_err(|_| DomainError::Validation {
                    field: "lorawan_payload.dev_addr",
                    reason: "invalid dev_addr bytes".to_string(),
                })?,
        );
    let fctrl = mac_payload[4];
    let frame_counter = u16::from_le_bytes([mac_payload[5], mac_payload[6]]) as u32;
    let fopts_len = (fctrl & 0x0F) as usize;
    let mut index = 7 + fopts_len;

    if index > mac_payload.len() {
        return Err(DomainError::Validation {
            field: "lorawan_payload.fopts",
            reason: "fopts length exceeds mac payload".to_string(),
        }
        .into());
    }

    let fport = if index < mac_payload.len() {
        let value = Some(mac_payload[index]);
        index += 1;
        value
    } else {
        None
    };

    let frm_payload = if index < mac_payload.len() {
        mac_payload[index..].to_vec()
    } else {
        Vec::new()
    };

    Ok(ParsedLorawanFrame::DataUplink {
        confirmed,
        dev_addr,
        fctrl,
        frame_counter,
        fport,
        frm_payload,
        mic,
    })
}

fn join_observed_audit(
    dev_eui: &Eui64,
    correlation_id: Option<String>,
    payload_size: usize,
) -> AuditRecord {
    let mut audit = AuditRecord::new(
        EventSource::Udp,
        "lorawan.join_request.observed",
        "uplink_frame",
        EventStatus::Accepted,
        "join request classified from uplink frame",
    )
    .with_metadata("payload_size", payload_size.to_string());
    audit.entity_id = Some(dev_eui.to_string());
    audit.correlation_id = correlation_id;
    audit
}

#[cfg(test)]
mod service_tests {
    use super::*;
    use crate::events::AuditRecord;
    use async_trait::async_trait;
    use maverick_domain::{
        AppKey, Device, DeviceKeys, Frequency, NwkKey, Rssi, Snr, SpreadingFactor,
    };
    use std::collections::HashMap;
    use std::sync::Mutex;

    // ── Stubs ────────────────────────────────────────────────────────────────

    struct StubAuditLog;
    #[async_trait]
    impl crate::ports::AuditLogWriter for StubAuditLog {
        async fn record(&self, _record: AuditRecord) -> crate::Result<()> {
            Ok(())
        }
    }

    struct StubSessionRepo {
        inner: Mutex<HashMap<u32, (Eui64, DeviceSession)>>,
    }
    impl StubSessionRepo {
        fn empty() -> Self {
            Self {
                inner: Mutex::new(HashMap::new()),
            }
        }
        fn with_session(dev_eui: Eui64, session: DeviceSession) -> Self {
            let mut map = HashMap::new();
            map.insert(session.dev_addr, (dev_eui, session));
            Self {
                inner: Mutex::new(map),
            }
        }
    }
    #[async_trait]
    impl SessionRepository for StubSessionRepo {
        async fn upsert_for_device(
            &self,
            dev_eui: Eui64,
            session: DeviceSession,
        ) -> crate::Result<()> {
            self.inner
                .lock()
                .unwrap()
                .insert(session.dev_addr, (dev_eui, session));
            Ok(())
        }
        async fn get_by_dev_eui(&self, _dev_eui: Eui64) -> crate::Result<Option<DeviceSession>> {
            Ok(None)
        }
        async fn get_by_dev_addr(
            &self,
            dev_addr: u32,
        ) -> crate::Result<Option<(Eui64, DeviceSession)>> {
            Ok(self.inner.lock().unwrap().get(&dev_addr).cloned())
        }
    }

    struct StubDeviceRepo {
        devices: Vec<Device>,
    }
    impl StubDeviceRepo {
        fn empty() -> Self {
            Self { devices: vec![] }
        }
        fn with_device(device: Device) -> Self {
            Self {
                devices: vec![device],
            }
        }
    }
    #[async_trait]
    impl DeviceRepository for StubDeviceRepo {
        async fn create(&self, d: Device) -> crate::Result<Device> {
            Ok(d)
        }
        async fn get_by_dev_eui(&self, dev_eui: Eui64) -> crate::Result<Option<Device>> {
            Ok(self.devices.iter().find(|d| d.dev_eui == dev_eui).cloned())
        }
        async fn update(&self, d: Device) -> crate::Result<Device> {
            Ok(d)
        }
        async fn delete(&self, _: Eui64) -> crate::Result<()> {
            Ok(())
        }
    }

    fn make_uplink(payload: Vec<u8>) -> UplinkFrame {
        UplinkFrame::new(
            Eui64::from([0xAA; 8]),
            payload,
            Rssi::new(-80),
            Snr::new(7.0),
            Frequency::new(868_100_000),
            SpreadingFactor::new(7).unwrap(),
            0,
            vec![],
        )
    }

    /// Build a valid 23-byte JoinRequest with correct MIC.
    fn build_join_request(
        app_key: &[u8; 16],
        app_eui: [u8; 8],
        dev_eui: [u8; 8],
        dev_nonce: u16,
    ) -> Vec<u8> {
        use aes::Aes128;
        use cmac::{Cmac, Mac};
        let mut payload = vec![0x00u8]; // MHDR: MType=JoinRequest
        payload.extend_from_slice(&app_eui);
        payload.extend_from_slice(&dev_eui);
        payload.extend_from_slice(&dev_nonce.to_le_bytes());
        // Compute MIC = CMAC(AppKey, payload[0..19])
        let mut mac = <Cmac<Aes128> as Mac>::new_from_slice(app_key).unwrap();
        mac.update(&payload);
        let result = mac.finalize().into_bytes();
        payload.extend_from_slice(&result[..4]);
        payload
    }

    fn make_service<S, A, R>(
        sessions: S,
        audit: A,
        devices: R,
    ) -> ProcessUplinkFrameService<S, A, R>
    where
        S: SessionRepository,
        A: crate::ports::AuditLogWriter,
        R: DeviceRepository,
    {
        ProcessUplinkFrameService::new(sessions, audit, EventBus::new(16), devices)
    }

    // ── Tests ─────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn join_request_unknown_device_rejected() {
        let app_key = [0x01u8; 16];
        let app_eui = [0xAA; 8];
        let dev_eui = [0xBB; 8];
        let payload = build_join_request(&app_key, app_eui, dev_eui, 1);

        let svc = make_service(
            StubSessionRepo::empty(),
            StubAuditLog,
            StubDeviceRepo::empty(),
        );
        let cmd = ProcessUplinkFrameCommand {
            uplink: make_uplink(payload),
            correlation_id: None,
        };
        let result = svc.process(cmd).await.unwrap();
        assert!(
            matches!(result, ProcessUplinkFrameOutcome::Rejected { reason } if reason.contains("unknown_device"))
        );
    }

    #[tokio::test]
    async fn join_request_accepted_with_valid_mic() {
        let app_key = [0x02u8; 16];
        let app_eui = [0xAA; 8];
        let dev_eui_bytes = [0xCC; 8];
        let payload = build_join_request(&app_key, app_eui, dev_eui_bytes, 42);

        let dev_eui = Eui64::from(dev_eui_bytes);
        let device = Device::new(
            dev_eui,
            Eui64::from(app_eui),
            DeviceKeys::new(AppKey::new(app_key), NwkKey::new([0u8; 16])),
        );
        let svc = make_service(
            StubSessionRepo::empty(),
            StubAuditLog,
            StubDeviceRepo::with_device(device),
        );
        let cmd = ProcessUplinkFrameCommand {
            uplink: make_uplink(payload),
            correlation_id: None,
        };
        let result = svc.process(cmd).await.unwrap();
        assert!(matches!(
            result,
            ProcessUplinkFrameOutcome::JoinAccepted { .. }
        ));
    }

    #[tokio::test]
    async fn join_request_invalid_mic_rejected() {
        let app_key = [0x03u8; 16];
        let app_eui = [0xAA; 8];
        let dev_eui_bytes = [0xDD; 8];
        // Build request with wrong MIC (last 4 bytes tampered)
        let mut payload = build_join_request(&app_key, app_eui, dev_eui_bytes, 1);
        let len = payload.len();
        payload[len - 1] ^= 0xFF; // flip last byte of MIC

        let dev_eui = Eui64::from(dev_eui_bytes);
        let device = Device::new(
            dev_eui,
            Eui64::from(app_eui),
            DeviceKeys::new(AppKey::new(app_key), NwkKey::new([0u8; 16])),
        );
        let svc = make_service(
            StubSessionRepo::empty(),
            StubAuditLog,
            StubDeviceRepo::with_device(device),
        );
        let cmd = ProcessUplinkFrameCommand {
            uplink: make_uplink(payload),
            correlation_id: None,
        };
        let result = svc.process(cmd).await.unwrap();
        assert!(
            matches!(result, ProcessUplinkFrameOutcome::Rejected { reason } if reason == "join_mic_invalid")
        );
    }

    #[tokio::test]
    async fn data_uplink_invalid_mic_rejected() {
        // Build a minimal DataUplink frame with a bad MIC
        let dev_addr: u32 = 0x01020304;
        let nwk_s_key = [0xABu8; 16];
        let dev_eui = Eui64::from([0xEE; 8]);
        let session = DeviceSession::new(dev_addr, [0u8; 16], nwk_s_key);

        // Unconfirmed DataUplink MHDR=0x40, FHDR=dev_addr(4) + fctrl(1) + fcnt(2) = 7 bytes,
        // no FPort, no payload, MIC=4 bytes. Total: 1 + 7 + 4 = 12 bytes.
        let payload = vec![
            0x40, 0x04, 0x03, 0x02, 0x01, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF,
        ];

        let svc = make_service(
            StubSessionRepo::with_session(dev_eui, session),
            StubAuditLog,
            StubDeviceRepo::empty(),
        );
        let cmd = ProcessUplinkFrameCommand {
            uplink: make_uplink(payload),
            correlation_id: None,
        };
        let result = svc.process(cmd).await.unwrap();
        assert!(
            matches!(result, ProcessUplinkFrameOutcome::Rejected { reason } if reason == "mic_invalid")
        );
    }
}
#[cfg(test)]
mod tests {
    use super::{parse_lorawan_frame, ParsedLorawanFrame};

    #[test]
    fn parses_join_request_frame() {
        let mut payload = vec![0x00];
        payload.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
        payload.extend_from_slice(&[8, 7, 6, 5, 4, 3, 2, 1]);
        payload.extend_from_slice(&0xCAFEu16.to_le_bytes());
        payload.extend_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]);

        let parsed = parse_lorawan_frame(&payload).expect("join frame must parse");

        match parsed {
            ParsedLorawanFrame::JoinRequest {
                app_eui,
                dev_eui,
                dev_nonce,
                mic,
            } => {
                assert_eq!(app_eui.as_bytes(), [1, 2, 3, 4, 5, 6, 7, 8]);
                assert_eq!(dev_eui.as_bytes(), [8, 7, 6, 5, 4, 3, 2, 1]);
                assert_eq!(dev_nonce, 0xCAFE);
                assert_eq!(mic, [0xAA, 0xBB, 0xCC, 0xDD]);
            }
            _ => panic!("expected join request"),
        }
    }

    #[test]
    fn parses_unconfirmed_data_uplink() {
        let mut payload = vec![0x40];
        payload.extend_from_slice(&0x26011BDAu32.to_le_bytes());
        payload.push(0x00);
        payload.extend_from_slice(&0x0012u16.to_le_bytes());
        payload.push(0x01);
        payload.extend_from_slice(&[0x10, 0x20, 0x30]);
        payload.extend_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF]);

        let parsed = parse_lorawan_frame(&payload).expect("data uplink must parse");

        match parsed {
            ParsedLorawanFrame::DataUplink {
                confirmed,
                dev_addr,
                frame_counter,
                fport,
                frm_payload,
                mic,
                ..
            } => {
                assert!(!confirmed);
                assert_eq!(dev_addr, 0x26011BDA);
                assert_eq!(frame_counter, 0x0012);
                assert_eq!(fport, Some(1));
                assert_eq!(frm_payload, vec![0x10, 0x20, 0x30]);
                assert_eq!(mic, [0xDE, 0xAD, 0xBE, 0xEF]);
            }
            _ => panic!("expected data uplink"),
        }
    }

    #[test]
    fn rejects_short_payload() {
        let error =
            parse_lorawan_frame(&[0x40, 0x01, 0x02]).expect_err("must reject short payload");
        assert!(error.to_string().contains("frame too short"));
    }
}
