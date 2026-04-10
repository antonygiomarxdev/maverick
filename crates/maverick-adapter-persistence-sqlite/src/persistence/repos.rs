//! Port implementations: sessions, uplinks, audit, storage pressure source.

use async_trait::async_trait;
use maverick_core::error::{AppError, AppResult};
use maverick_core::ports::{
    AuditRecord, AuditSink, SessionRepository, UplinkRecord, UplinkRepository,
};
use maverick_core::storage::{
    StoragePressureLevel, StoragePressureSnapshot, StoragePressureSource,
};
use maverick_domain::{DevAddr, SessionSnapshot};
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
                        &record.payload[..],
                    ],
                )?;
                p.prune_uplinks_sql(conn)?;
                p.prune_hard_limit_circular_sql(conn)?;
                Ok(())
            })
        })
        .await
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
