//! LNS/session gating around uplink ingest (pending registration, rate limits).

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use maverick_adapter_persistence_sqlite::SqlitePersistence;
use maverick_core::error::AppError;
use maverick_core::ports::{AuditRecord, AuditSink, SessionRepository, UplinkObservation};
use maverick_core::use_cases::IngestUplink;
use maverick_domain::identifiers::Eui64;
use maverick_domain::GatewayEui;
use serde_json::json;

/// Sliding window per gateway / minute for autoprovision `pending` inserts (`0` = unlimited).
pub(crate) fn autoprovision_rate_allow(gw: &GatewayEui, limit_per_minute: u32) -> bool {
    if limit_per_minute == 0 {
        return true;
    }
    type GatewayMinuteKey = ([u8; 8], u64);
    static BUCKET: OnceLock<Mutex<HashMap<GatewayMinuteKey, u32>>> = OnceLock::new();
    let bucket = BUCKET.get_or_init(|| Mutex::new(HashMap::new()));
    let minute = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() / 60)
        .unwrap_or(0);
    let mut g = bucket.lock().expect("autoprovision rate mutex poisoned");
    g.retain(|&(_, m), _| m + 12 >= minute);
    let key = (gw.0 .0, minute);
    let entry = g.entry(key).or_insert(0);
    if *entry >= limit_per_minute {
        return false;
    }
    *entry += 1;
    true
}

pub(crate) fn format_eui64_hex(eui: &Eui64) -> String {
    use std::fmt::Write as _;
    eui.0.iter().fold(String::with_capacity(16), |mut acc, b| {
        let _ = write!(&mut acc, "{b:02X}");
        acc
    })
}

pub(crate) async fn ingest_uplink_with_lns_guard(
    store: &Arc<SqlitePersistence>,
    ingest: &IngestUplink,
    obs: UplinkObservation,
) -> Result<(), AppError> {
    let sessions: Arc<dyn SessionRepository> = store.clone();
    match sessions.get_by_dev_addr(obs.dev_addr).await {
        Ok(Some(_)) => ingest.execute(obs).await,
        Ok(None) => {
            let policy = store.lns_autoprovision_policy()?;
            if !policy.enabled {
                return Err(AppError::Domain(
                    "no session for DevAddr; autoprovision disabled (lns-config.toml)".to_string(),
                ));
            }
            if !autoprovision_rate_allow(&obs.gateway_eui, policy.rate_limit_per_gateway_per_minute)
            {
                let audit: Arc<dyn AuditSink> = store.clone();
                audit
                    .emit(AuditRecord {
                        source: "edge".to_string(),
                        operation: "ingest_uplink".to_string(),
                        entity_type: "device".to_string(),
                        entity_id: Some(format!("{:08x}", obs.dev_addr.0)),
                        outcome: "autoprovision_rate_limited".to_string(),
                        metadata: Some(json!({
                            "gateway_eui": format_eui64_hex(&obs.gateway_eui.0),
                        })),
                    })
                    .await?;
                return Err(AppError::Domain(
                    "autoprovision rate limit exceeded for this gateway".to_string(),
                ));
            }
            store.lns_upsert_pending(obs.dev_addr, obs.gateway_eui)?;
            let audit: Arc<dyn AuditSink> = store.clone();
            audit
                .emit(AuditRecord {
                    source: "edge".to_string(),
                    operation: "ingest_uplink".to_string(),
                    entity_type: "device".to_string(),
                    entity_id: Some(format!("{:08x}", obs.dev_addr.0)),
                    outcome: "pending_registration".to_string(),
                    metadata: Some(json!({
                        "gateway_eui": format_eui64_hex(&obs.gateway_eui.0),
                        "hint": "add device to lns-config.toml then: maverick-edge config load",
                    })),
                })
                .await?;
            Err(AppError::Domain(
                "DevAddr unknown: pending registration; configure device then config load"
                    .to_string(),
            ))
        }
        Err(e) => Err(e),
    }
}
