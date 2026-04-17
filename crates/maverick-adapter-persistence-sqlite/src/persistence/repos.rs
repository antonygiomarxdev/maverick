//! Port implementations: sessions, uplinks, audit, storage pressure source.

use async_trait::async_trait;
use maverick_core::error::{AppError, AppResult};
use maverick_core::ports::{
    AuditRecord, AuditSink, DownlinkEnqueue, DownlinkItem, DownlinkRepository, SessionRepository,
    UplinkRecord, UplinkRepository,
};
use maverick_core::storage::{
    StoragePressureLevel, StoragePressureSnapshot, StoragePressureSource,
};
use maverick_domain::identifiers::Eui64;
use maverick_domain::{DevAddr, DevEui, SessionSnapshot};
use rusqlite::params;

use crate::persisted_device_class::PersistedDeviceClassTag;
use crate::schema;

use super::sql::{now_ms, row_to_session};
use super::SqlitePersistence;

#[async_trait]
impl SessionRepository for SqlitePersistence {
    async fn get_by_dev_addr(&self, dev_addr: DevAddr) -> AppResult<Option<SessionSnapshot>> {
        let this = self.clone();
        let key = dev_addr.0 as i64;
        this.run_blocking(move |p| {
            p.run_with_busy_retry(|conn| {
                let sql = schema::sql_select_session_by_dev_addr();
                let mut stmt = conn.prepare(sql.as_str())?;
                match stmt.query_row(params![key], row_to_session) {
                    Ok(s) => Ok(Some(s)),
                    Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                    Err(e) => Err(e),
                }
            })
        })
        .await
    }

    async fn upsert(&self, session: &SessionSnapshot) -> AppResult<()> {
        let session = session.clone();
        let this = self.clone();
        this.run_blocking(move |p| {
            p.run_with_busy_retry(|conn| {
                let ts = now_ms().0;
                let sql = schema::sql_upsert_session();
                let region = session.region.to_string();
                let class_tag = PersistedDeviceClassTag::from(session.class);
                conn.execute(
                    sql.as_str(),
                    params![
                        session.dev_addr.0 as i64,
                        &session.dev_eui.0 .0[..],
                        region,
                        class_tag.as_str(),
                        session.uplink_frame_counter as i64,
                        session.downlink_frame_counter as i64,
                        ts,
                        session.application_id.clone(),
                        &session.nwk_s_key[..],
                        &session.app_s_key[..],
                    ],
                )?;
                p.prune_sessions_lru_sql(conn)?;
                p.prune_hard_limit_circular_sql(conn)?;
                Ok(())
            })
        })
        .await
    }
}

#[async_trait]
impl UplinkRepository for SqlitePersistence {
    async fn append(&self, record: &UplinkRecord) -> AppResult<()> {
        let record = record.clone();
        let this = self.clone();
        this.run_blocking(move |p| {
            p.run_with_busy_retry(|conn| {
                let sql = schema::sql_insert_uplink();
                conn.execute(
                    sql.as_str(),
                    params![
                        record.dev_addr.0 as i64,
                        record.f_cnt as i64,
                        record.received_at_ms,
                        &record.payload[..],
                        record.application_id.clone(),
                        record.payload_decrypted.as_deref(),
                    ],
                )?;
                p.prune_uplinks_sql(conn)?;
                p.prune_hard_limit_circular_sql(conn)?;
                Ok(())
            })
        })
        .await
    }

    async fn is_duplicate(&self, dev_addr: DevAddr, f_cnt: u32, window_ms: i64) -> AppResult<bool> {
        let this = self.clone();
        let key = dev_addr.0 as i64;
        let fcnt = f_cnt as i64;
        this.run_blocking(move |p| {
            p.run_with_busy_retry(|conn| {
                use std::time::{SystemTime, UNIX_EPOCH};
                let now_ms = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_millis().min(i64::MAX as u128) as i64)
                    .unwrap_or(0);
                let cutoff_ms = now_ms - window_ms;
                let sql = schema::sql_check_uplink_dedup();
                let count: i64 =
                    conn.query_row(sql.as_str(), rusqlite::params![key, fcnt, cutoff_ms], |r| {
                        r.get(0)
                    })?;
                Ok(count > 0)
            })
        })
        .await
    }
}

#[async_trait]
impl DownlinkRepository for SqlitePersistence {
    async fn enqueue(&self, item: &DownlinkEnqueue) -> AppResult<u64> {
        let item = item.clone();
        let this = self.clone();
        this.run_blocking(move |p| {
            p.run_with_busy_retry(|conn| {
                let ts = now_ms().0;
                let sql = schema::sql_insert_downlink();
                conn.execute(
                    sql.as_str(),
                    params![
                        &item.dev_eui.0 .0[..],
                        0i64,
                        item.f_port as i64,
                        &item.payload[..],
                        item.confirmed as i64,
                        0i64,
                        ts,
                        0i64,
                    ],
                )?;
                Ok(conn.last_insert_rowid() as u64)
            })
        })
        .await
    }

    async fn dequeue_oldest(&self, dev_eui: &DevEui, limit: usize) -> AppResult<Vec<DownlinkItem>> {
        let dev_eui = *dev_eui;
        let this = self.clone();
        this.run_blocking(move |p| {
            p.run_with_busy_retry(|conn| {
                let sql = schema::sql_select_pending_downlinks();
                let mut stmt = conn.prepare(sql.as_str())?;
                let rows = stmt.query_map(params![&dev_eui.0 .0[..], limit as i64], |row| {
                    let dev_eui_bytes: Vec<u8> = row.get(1)?;
                    let dev_addr: i64 = row.get(2)?;
                    Ok(DownlinkItem {
                        id: row.get::<_, i64>(0)? as u64,
                        dev_eui: DevEui(Eui64(dev_eui_bytes.try_into().map_err(|_| {
                            rusqlite::Error::FromSqlConversionFailure(
                                1,
                                rusqlite::types::Type::Blob,
                                "Invalid EUI64 length".into(),
                            )
                        })?)),
                        dev_addr: DevAddr(dev_addr as u32),
                        f_port: row.get::<_, i64>(3)? as u8,
                        payload: row.get(4)?,
                        confirmed: row.get::<_, i64>(5)? != 0,
                        ack_flag: row.get::<_, i64>(6)? != 0,
                        enqueued_at_ms: row.get(7)?,
                        frame_counter: row.get::<_, i64>(8)? as u32,
                    })
                })?;
                rows.collect::<Result<Vec<_>, _>>()
            })
        })
        .await
    }

    async fn mark_transmitted(&self, id: u64) -> AppResult<()> {
        let this = self.clone();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
        this.run_blocking(move |p| {
            p.run_with_busy_retry(|conn| {
                let sql = schema::sql_update_downlink_status();
                conn.execute(sql.as_str(), params!["transmitted", now, id as i64])?;
                Ok(())
            })
        })
        .await
    }

    async fn mark_failed(&self, id: u64) -> AppResult<()> {
        let this = self.clone();
        this.run_blocking(move |p| {
            p.run_with_busy_retry(|conn| {
                let sql = schema::sql_update_downlink_status();
                conn.execute(
                    sql.as_str(),
                    params!["failed", rusqlite::types::Null, id as i64],
                )?;
                Ok(())
            })
        })
        .await
    }

    async fn get_pending_for_dev(&self, dev_eui: &DevEui) -> AppResult<Vec<DownlinkItem>> {
        self.dequeue_oldest(dev_eui, 10).await
    }
}

#[async_trait]
impl AuditSink for SqlitePersistence {
    async fn emit(&self, record: AuditRecord) -> AppResult<()> {
        let meta_str = record
            .metadata
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|e| AppError::InvalidInput(format!("audit metadata json: {e}")))?;
        let this = self.clone();
        this.run_blocking(move |p| {
            p.run_with_busy_retry(|conn| {
                let sql = schema::sql_insert_audit_event();
                conn.execute(
                    sql.as_str(),
                    params![
                        record.source,
                        record.operation,
                        record.entity_type,
                        record.entity_id,
                        record.outcome,
                        meta_str,
                        now_ms().0,
                    ],
                )?;
                p.prune_audit_sql(conn)?;
                p.prune_hard_limit_circular_sql(conn)?;
                Ok(())
            })
        })
        .await
    }
}

#[async_trait]
impl StoragePressureSource for SqlitePersistence {
    async fn pressure_snapshot(&self) -> StoragePressureSnapshot {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.pressure_snapshot_blocking())
            .await
            .unwrap_or_else(|e| StoragePressureSnapshot {
                level: StoragePressureLevel::Critical,
                db_bytes: 0,
                total_disk_bytes: None,
                detail: Some(format!("pressure task failed: {e}")),
            })
    }
}
