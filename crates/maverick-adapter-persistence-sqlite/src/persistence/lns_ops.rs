//! LNS declarative config → SQLite sync and pending-device bookkeeping.

use maverick_core::error::{AppError, AppResult};
use maverick_core::lns_config::{
    parse_hex_16, parse_hex_32, parse_hex_dev_addr, parse_hex_dev_eui, ActivationMode,
    LnsConfigDocument,
};
use maverick_domain::identifiers::Eui64;
use maverick_domain::{DevAddr, DevEui, DeviceClass, GatewayEui, RegionId, SessionSnapshot};
use rusqlite::{params, Connection};

use crate::persisted_device_class::PersistedDeviceClassTag;
use crate::schema;

use super::sql::{now_ms, row_to_session};
use super::SqlitePersistence;

impl SqlitePersistence {
    /// Transactional sync from declarative LNS config into SQLite (applications/devices/meta/sessions).
    pub fn apply_lns_config(&self, doc: &LnsConfigDocument) -> AppResult<()> {
        doc.validate().map_err(AppError::InvalidInput)?;
        self.run_with_busy_retry(|conn| apply_lns_config_inner(self, conn, doc))
    }

    /// Insert or refresh a pending row for unknown `DevAddr` (autoprovision path).
    pub fn lns_upsert_pending(&self, dev_addr: DevAddr, gateway_eui: GatewayEui) -> AppResult<()> {
        self.run_with_busy_retry(|conn| {
            let ts = now_ms().0;
            conn.execute(
                "INSERT INTO lns_pending (dev_addr, gateway_eui, first_seen_ms) VALUES (?1, ?2, ?3)
                 ON CONFLICT(dev_addr) DO UPDATE SET gateway_eui = excluded.gateway_eui",
                params![dev_addr.0 as i64, &gateway_eui.0 .0[..], ts],
            )?;
            Ok(())
        })
    }

    pub fn lns_delete_pending(&self, dev_addr: DevAddr) -> AppResult<()> {
        self.run_with_busy_retry(|conn| {
            conn.execute(
                "DELETE FROM lns_pending WHERE dev_addr = ?1",
                params![dev_addr.0 as i64],
            )?;
            Ok(())
        })
    }

    /// Operator approval: promote a pending device into `lns_devices` + `sessions`.
    pub fn lns_approve_device(
        &self,
        dev_eui_hex: &str,
        dev_addr_hex: &str,
        application_id: &str,
        region: RegionId,
    ) -> AppResult<()> {
        let dev_eui_b = parse_hex_dev_eui(dev_eui_hex).map_err(AppError::InvalidInput)?;
        let dev_addr_u = parse_hex_dev_addr(dev_addr_hex).map_err(AppError::InvalidInput)?;
        let dev_addr = DevAddr(dev_addr_u);
        self.run_with_busy_retry(|conn| {
            let tx = conn.transaction()?;
            let ts = now_ms().0;
            tx.execute(
                "INSERT INTO lns_devices (dev_eui, dev_addr, activation_mode, application_id, region, enabled, join_eui, app_key, nwk_key, apps_key, nwks_key)
                 VALUES (?1, ?2, 'abp', ?3, ?4, 1, NULL, NULL, NULL, NULL, NULL)
                 ON CONFLICT(dev_eui) DO UPDATE SET
                   dev_addr = excluded.dev_addr,
                   activation_mode = excluded.activation_mode,
                   application_id = excluded.application_id,
                   region = excluded.region,
                   enabled = 1",
                params![
                    &dev_eui_b[..],
                    dev_addr.0 as i64,
                    application_id,
                    region.to_string(),
                ],
            )?;
            let existing = {
                let sql = schema::sql_select_session_by_dev_addr();
                let mut stmt = tx.prepare(sql.as_str())?;
                match stmt.query_row(params![dev_addr.0 as i64], row_to_session) {
                    Ok(s) => Some(s),
                    Err(rusqlite::Error::QueryReturnedNoRows) => None,
                    Err(e) => return Err(e),
                }
            };
            let uplink_fc = existing
                .as_ref()
                .map(|s| s.uplink_frame_counter)
                .unwrap_or(0);
            let downlink_fc = existing
                .as_ref()
                .map(|s| s.downlink_frame_counter)
                .unwrap_or(0);
            let nwk_s_key = existing.as_ref().map(|s| s.nwk_s_key).unwrap_or([0u8; 16]);
            let app_s_key = existing.as_ref().map(|s| s.app_s_key).unwrap_or([0u8; 16]);
            let session = SessionSnapshot {
                dev_eui: DevEui(Eui64(dev_eui_b)),
                dev_addr,
                region,
                class: DeviceClass::ClassA,
                uplink_frame_counter: uplink_fc,
                downlink_frame_counter: downlink_fc,
                application_id: Some(application_id.to_string()),
                nwk_s_key,
                app_s_key,
            };
            let sql = schema::sql_upsert_session();
            let class_tag = PersistedDeviceClassTag::from(session.class);
            tx.execute(
                sql.as_str(),
                params![
                    session.dev_addr.0 as i64,
                    &session.dev_eui.0 .0[..],
                    session.region.to_string(),
                    class_tag.as_str(),
                    session.uplink_frame_counter as i64,
                    session.downlink_frame_counter as i64,
                    ts,
                    session.application_id.clone(),
                    &session.nwk_s_key[..],
                    &session.app_s_key[..],
                ],
            )?;
            tx.execute(
                "DELETE FROM lns_pending WHERE dev_addr = ?1",
                params![dev_addr.0 as i64],
            )?;
            tx.commit()?;
            Ok(())
        })
    }

    /// Load autoprovision policy from `lns_meta` (defaults if missing).
    pub fn lns_autoprovision_policy(&self) -> AppResult<LnsAutoprovisionMeta> {
        self.run_with_busy_retry(read_lns_meta)
    }

    pub fn lns_list_applications(&self) -> AppResult<Vec<LnsApplicationRow>> {
        self.run_with_busy_retry(|conn| {
            let mut stmt =
                conn.prepare("SELECT id, name, default_region FROM lns_applications ORDER BY id")?;
            let rows = stmt.query_map([], |r| {
                Ok(LnsApplicationRow {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    default_region: r.get(2)?,
                })
            })?;
            let mut v = Vec::new();
            for row in rows {
                v.push(row?);
            }
            Ok(v)
        })
    }

    pub fn lns_list_devices(&self) -> AppResult<Vec<LnsDeviceListRow>> {
        self.run_with_busy_retry(|conn| {
            let mut stmt = conn.prepare(
                "SELECT d.dev_eui, d.dev_addr, d.activation_mode, d.application_id, d.region, d.enabled,
                        s.updated_at_ms,
                        (SELECT COUNT(*) FROM uplinks u WHERE u.dev_addr = d.dev_addr)
                 FROM lns_devices d
                 LEFT JOIN sessions s ON s.dev_addr = d.dev_addr
                 ORDER BY d.application_id, d.dev_eui",
            )?;
            let rows = stmt.query_map([], |r| {
                let dev_eui: Vec<u8> = r.get(0)?;
                let dev_addr: Option<i64> = r.get(1)?;
                let activation_mode: String = r.get(2)?;
                let application_id: String = r.get(3)?;
                let region: String = r.get(4)?;
                let enabled: bool = r.get::<_, i64>(5)? != 0;
                let last_seen_timestamp: Option<i64> = r.get(6)?;
                let uplink_count: Option<i64> = r.get(7)?;
                Ok(LnsDeviceListRow {
                    activation_mode,
                    dev_eui_hex: hex_upper_8(&dev_eui),
                    dev_addr_hex: dev_addr.map(|a| format!("{:08X}", a as u32)),
                    application_id,
                    region,
                    enabled,
                    last_seen_timestamp,
                    uplink_count,
                })
            })?;
            let mut v = Vec::new();
            for row in rows {
                v.push(row?);
            }
            Ok(v)
        })
    }

    pub fn lns_list_pending(&self) -> AppResult<Vec<LnsPendingRow>> {
        self.run_with_busy_retry(|conn| {
            let mut stmt = conn.prepare(
                "SELECT dev_addr, gateway_eui, first_seen_ms FROM lns_pending ORDER BY first_seen_ms DESC",
            )?;
            let rows = stmt.query_map([], |r| {
                let dev_addr: i64 = r.get(0)?;
                let gw: Vec<u8> = r.get(1)?;
                Ok(LnsPendingRow {
                    dev_addr_hex: format!("{:08X}", dev_addr as u32),
                    gateway_eui_hex: hex_upper_8(&gw),
                    first_seen_ms: r.get(2)?,
                })
            })?;
            let mut v = Vec::new();
            for row in rows {
                v.push(row?);
            }
            Ok(v)
        })
    }

    pub fn lns_show_device(&self, dev_eui_hex: &str) -> AppResult<Option<LnsDeviceShowRow>> {
        self.run_with_busy_retry(|conn| {
            let dev_eui_b = parse_hex_dev_eui(dev_eui_hex)
                .map_err(|e| rusqlite::Error::InvalidParameterName(e))?;
            let mut stmt = conn.prepare(
                "SELECT d.dev_eui, d.dev_addr, d.activation_mode, d.application_id, d.region, d.enabled,
                        s.updated_at_ms,
                        (SELECT COUNT(*) FROM uplinks u WHERE u.dev_addr = d.dev_addr),
                        s.nwk_s_key, s.app_s_key
                 FROM lns_devices d
                 LEFT JOIN sessions s ON s.dev_addr = d.dev_addr
                 WHERE d.dev_eui = ?1",
            )?;
            let row = stmt.query_row(params![&dev_eui_b[..]], |r| {
                let dev_eui: Vec<u8> = r.get(0)?;
                let dev_addr: Option<i64> = r.get(1)?;
                let activation_mode: String = r.get(2)?;
                let application_id: String = r.get(3)?;
                let region: String = r.get(4)?;
                let enabled: bool = r.get::<_, i64>(5)? != 0;
                let last_seen_timestamp: Option<i64> = r.get(6)?;
                let uplink_count: Option<i64> = r.get(7)?;
                let nwk_s_key: Option<Vec<u8>> = r.get(8)?;
                let app_s_key: Option<Vec<u8>> = r.get(9)?;
                Ok(LnsDeviceShowRow {
                    activation_mode,
                    dev_eui_hex: hex_upper_8(&dev_eui),
                    dev_addr_hex: dev_addr.map(|a| format!("{:08X}", a as u32)),
                    application_id,
                    region,
                    enabled,
                    last_seen_timestamp,
                    uplink_count,
                    nwk_s_key_hex: nwk_s_key.as_ref().map(|v| hex_upper_16(v.as_slice())),
                    app_s_key_hex: app_s_key.as_ref().map(|v| hex_upper_16(v.as_slice())),
                })
            });
            match row {
                Ok(r) => Ok(Some(r)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(e),
            }
        })
    }
}

fn hex_upper_8(bytes: &[u8]) -> String {
    if bytes.len() != 8 {
        return format!("invalid_len_{}", bytes.len());
    }
    bytes.iter().fold(String::with_capacity(16), |mut acc, b| {
        use std::fmt::Write as _;
        let _ = write!(&mut acc, "{b:02X}");
        acc
    })
}

fn hex_upper_16(bytes: &[u8]) -> String {
    if bytes.len() != 16 {
        return format!("invalid_len_{}", bytes.len());
    }
    bytes.iter().fold(String::with_capacity(32), |mut acc, b| {
        use std::fmt::Write as _;
        let _ = write!(&mut acc, "{b:02X}");
        acc
    })
}

/// Row for CLI / JSON (`dev_eui` as hex string).
#[derive(Debug, Clone, serde::Serialize)]
pub struct LnsApplicationRow {
    pub id: String,
    pub name: String,
    pub default_region: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct LnsDeviceListRow {
    pub activation_mode: String,
    pub dev_eui_hex: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dev_addr_hex: Option<String>,
    pub application_id: String,
    pub region: String,
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_seen_timestamp: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uplink_count: Option<i64>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct LnsDeviceShowRow {
    pub activation_mode: String,
    pub dev_eui_hex: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dev_addr_hex: Option<String>,
    pub application_id: String,
    pub region: String,
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_seen_timestamp: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uplink_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nwk_s_key_hex: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_s_key_hex: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct LnsPendingRow {
    pub dev_addr_hex: String,
    pub gateway_eui_hex: String,
    pub first_seen_ms: i64,
}

fn read_lns_meta(conn: &mut Connection) -> Result<LnsAutoprovisionMeta, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT autoprovision_enabled, rate_limit_per_gateway_per_minute, pending_ttl_secs FROM lns_meta WHERE id = 1",
    )?;
    let mut rows = stmt.query([])?;
    if let Some(row) = rows.next()? {
        Ok(LnsAutoprovisionMeta {
            enabled: row.get::<_, i64>(0)? != 0,
            rate_limit_per_gateway_per_minute: row.get::<_, i64>(1)? as u32,
            pending_ttl_secs: row.get::<_, i64>(2)? as u64,
        })
    } else {
        Ok(LnsAutoprovisionMeta::default())
    }
}

/// Policy row mirrored from config (used by ingest autoprovision).
#[derive(Debug, Clone, serde::Serialize)]
pub struct LnsAutoprovisionMeta {
    pub enabled: bool,
    pub rate_limit_per_gateway_per_minute: u32,
    pub pending_ttl_secs: u64,
}

impl Default for LnsAutoprovisionMeta {
    fn default() -> Self {
        Self {
            enabled: true,
            rate_limit_per_gateway_per_minute: 10,
            pending_ttl_secs: 86_400,
        }
    }
}

fn apply_lns_config_inner(
    p: &SqlitePersistence,
    conn: &mut Connection,
    doc: &LnsConfigDocument,
) -> Result<(), rusqlite::Error> {
    let ts = now_ms().0;
    let tx = conn.transaction()?;
    tx.execute("DELETE FROM lns_applications", [])?;
    for app in &doc.applications {
        tx.execute(
            "INSERT INTO lns_applications (id, name, default_region) VALUES (?1, ?2, ?3)",
            params![&app.id, &app.name, &app.default_region],
        )?;
    }
    tx.execute("DELETE FROM lns_devices", [])?;
    for d in &doc.devices {
        let dev_eui_b = parse_hex_dev_eui(&d.dev_eui)
            .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        let activation_mode_str = match d.activation_mode {
            ActivationMode::Otaa => "otaa",
            ActivationMode::Abp => "abp",
        };
        let dev_addr_sql: Option<i64> = match d.activation_mode {
            ActivationMode::Abp => {
                let addr_str = d.dev_addr.as_ref().ok_or_else(|| {
                    rusqlite::Error::InvalidParameterName("abp dev_addr missing".to_string())
                })?;
                let u = parse_hex_dev_addr(addr_str)
                    .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
                Some(u as i64)
            }
            ActivationMode::Otaa => d.dev_addr.as_ref().and_then(|s| {
                let t = s.trim();
                if t.is_empty() {
                    None
                } else {
                    parse_hex_dev_addr(t).ok().map(|u| u as i64)
                }
            }),
        };
        type OptJoin = Option<[u8; 8]>;
        type OptKey16 = Option<[u8; 16]>;
        let (join_eui, app_key, nwk_key): (OptJoin, OptKey16, OptKey16) =
            if let Some(ref k) = d.otaa {
                let j = parse_hex_16(&k.join_eui)
                    .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
                let ak = parse_hex_32(&k.app_key)
                    .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
                let nk = k
                    .nwk_key
                    .as_ref()
                    .map(|s| {
                        parse_hex_32(s)
                            .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))
                    })
                    .transpose()?;
                (Some(j), Some(ak), nk)
            } else {
                (None, None, None)
            };
        let (apps_key, nwks_key): (OptKey16, OptKey16) = if let Some(ref abp) = d.abp {
            let a = abp
                .apps_key
                .as_ref()
                .filter(|s| !s.trim().is_empty())
                .map(|s| {
                    parse_hex_32(s)
                        .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))
                })
                .transpose()?;
            let n = abp
                .nwks_key
                .as_ref()
                .filter(|s| !s.trim().is_empty())
                .map(|s| {
                    parse_hex_32(s)
                        .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))
                })
                .transpose()?;
            (a, n)
        } else {
            (None, None)
        };
        tx.execute(
            "INSERT INTO lns_devices (dev_eui, dev_addr, activation_mode, application_id, region, enabled, join_eui, app_key, nwk_key, apps_key, nwks_key)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                &dev_eui_b[..],
                dev_addr_sql,
                activation_mode_str,
                &d.application_id,
                &d.region,
                if d.enabled { 1 } else { 0 },
                join_eui.as_ref().map(|b| b.as_slice()),
                app_key.as_ref().map(|b| b.as_slice()),
                nwk_key.as_ref().map(|b| b.as_slice()),
                apps_key.as_ref().map(|b| b.as_slice()),
                nwks_key.as_ref().map(|b| b.as_slice()),
            ],
        )?;
    }
    tx.execute(
        "INSERT INTO lns_meta (id, autoprovision_enabled, rate_limit_per_gateway_per_minute, pending_ttl_secs)
         VALUES (1, ?1, ?2, ?3)
         ON CONFLICT(id) DO UPDATE SET
           autoprovision_enabled = excluded.autoprovision_enabled,
           rate_limit_per_gateway_per_minute = excluded.rate_limit_per_gateway_per_minute,
           pending_ttl_secs = excluded.pending_ttl_secs",
        params![
            if doc.autoprovision.enabled { 1 } else { 0 },
            doc.autoprovision.rate_limit_per_gateway_per_minute as i64,
            doc.autoprovision.pending_ttl_secs as i64,
        ],
    )?;
    tx.execute(
        "DELETE FROM sessions WHERE dev_addr NOT IN (SELECT dev_addr FROM lns_devices WHERE enabled = 1 AND dev_addr IS NOT NULL)",
        [],
    )?;
    tx.execute(
        "DELETE FROM lns_pending WHERE dev_addr IN (SELECT dev_addr FROM lns_devices WHERE enabled = 1 AND dev_addr IS NOT NULL)",
        [],
    )?;
    for d in &doc.devices {
        if !d.enabled {
            continue;
        }
        let dev_addr_u = match d.activation_mode {
            ActivationMode::Abp => {
                let addr_str = d.dev_addr.as_ref().ok_or_else(|| {
                    rusqlite::Error::InvalidParameterName("abp dev_addr missing".to_string())
                })?;
                parse_hex_dev_addr(addr_str)
                    .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?
            }
            ActivationMode::Otaa => match d.dev_addr.as_ref() {
                None => continue,
                Some(s) => {
                    let t = s.trim();
                    if t.is_empty() {
                        continue;
                    }
                    match parse_hex_dev_addr(t) {
                        Ok(u) => u,
                        Err(_) => continue,
                    }
                }
            },
        };
        let dev_addr = DevAddr(dev_addr_u);
        let dev_eui_b = parse_hex_dev_eui(&d.dev_eui)
            .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        let region: RegionId =
            d.region
                .parse()
                .map_err(|e: maverick_domain::region::UnknownRegionError| {
                    rusqlite::Error::InvalidParameterName(e.to_string())
                })?;
        let sql_sel = schema::sql_select_session_by_dev_addr();
        let existing = {
            let mut stmt = tx.prepare(sql_sel.as_str())?;
            match stmt.query_row(params![dev_addr.0 as i64], row_to_session) {
                Ok(s) => Some(s),
                Err(rusqlite::Error::QueryReturnedNoRows) => None,
                Err(e) => return Err(e),
            }
        };
        let uplink_fc = existing
            .as_ref()
            .map(|s| s.uplink_frame_counter)
            .unwrap_or(0);
        let downlink_fc = existing
            .as_ref()
            .map(|s| s.downlink_frame_counter)
            .unwrap_or(0);
        // Use session keys from ABP config if present, then fall back to existing session, then zero.
        let nwk_s_key = d
            .abp
            .as_ref()
            .and_then(|abp| abp.nwks_key.as_ref())
            .filter(|s| !s.trim().is_empty())
            .and_then(|s| parse_hex_32(s).ok())
            .or_else(|| existing.as_ref().map(|s| s.nwk_s_key))
            .unwrap_or([0u8; 16]);
        let app_s_key = d
            .abp
            .as_ref()
            .and_then(|abp| abp.apps_key.as_ref())
            .filter(|s| !s.trim().is_empty())
            .and_then(|s| parse_hex_32(s).ok())
            .or_else(|| existing.as_ref().map(|s| s.app_s_key))
            .unwrap_or([0u8; 16]);
        let session = SessionSnapshot {
            dev_eui: DevEui(Eui64(dev_eui_b)),
            dev_addr,
            region,
            class: DeviceClass::ClassA,
            uplink_frame_counter: uplink_fc,
            downlink_frame_counter: downlink_fc,
            application_id: Some(d.application_id.clone()),
            nwk_s_key,
            app_s_key,
        };
        let sql_up = schema::sql_upsert_session();
        let class_tag = PersistedDeviceClassTag::from(session.class);
        tx.execute(
            sql_up.as_str(),
            params![
                session.dev_addr.0 as i64,
                &session.dev_eui.0 .0[..],
                session.region.to_string(),
                class_tag.as_str(),
                session.uplink_frame_counter as i64,
                session.downlink_frame_counter as i64,
                ts,
                session.application_id.clone(),
                &session.nwk_s_key[..],
                &session.app_s_key[..],
            ],
        )?;
    }
    tx.commit()?;
    p.prune_sessions_lru_sql(conn)?;
    p.prune_hard_limit_circular_sql(conn)?;
    Ok(())
}
