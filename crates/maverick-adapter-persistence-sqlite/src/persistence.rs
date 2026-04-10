use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use maverick_core::error::{AppError, AppResult};
use maverick_core::ports::{
    AuditRecord, AuditSink, SessionRepository, UplinkRecord, UplinkRepository,
};
use maverick_core::storage::{
    StoragePolicy, StoragePressureLevel, StoragePressureSnapshot, StoragePressureSource,
};
use maverick_domain::{DevAddr, DevEui, RegionId, SessionSnapshot};
use rusqlite::{params, Connection};

use crate::diag::{
    SQLITE_BUSY_RETRIES_EXHAUSTED, SQLITE_MUTEX_POISONED, STORED_FIELD_DEVICE_CLASS,
    STORED_FIELD_REGION,
};
use crate::limits::{
    BUSY_RETRY_BACKOFF_BASE_MS, DEV_EUI_BYTE_LEN, DISK_RATIO_HARD_LIMIT_ENTER,
    DISK_RATIO_HARD_LIMIT_TARGET, HARD_TRIM_AUDIT_BATCH, HARD_TRIM_MAX_ROUNDS,
    HARD_TRIM_SESSION_BATCH, HARD_TRIM_UPLINK_BATCH, TIER_FILL_CRITICAL_RATIO,
    TIER_FILL_ELEVATED_RATIO,
};
use crate::persisted_device_class::PersistedDeviceClassTag;
use crate::schema::{self, names};
use crate::sqlite_op::SqliteOperation;

/// Options for SQLite busy handling and optional disk capacity hint (pressure ratios).
#[derive(Debug, Clone)]
pub struct SqlitePersistenceOptions {
    pub busy_timeout_ms: u64,
    pub busy_retry_attempts: u32,
    pub total_disk_bytes: Option<u64>,
}

impl Default for SqlitePersistenceOptions {
    fn default() -> Self {
        Self {
            busy_timeout_ms: 5_000,
            busy_retry_attempts: 6,
            total_disk_bytes: None,
        }
    }
}

/// SQLite-backed persistence for edge runtime; implements core ports only.
#[derive(Clone)]
pub struct SqlitePersistence {
    inner: Arc<Inner>,
}

struct Inner {
    path: PathBuf,
    policy: StoragePolicy,
    options: SqlitePersistenceOptions,
    conn: std::sync::Mutex<Connection>,
}

impl SqlitePersistence {
    pub fn open(
        path: impl AsRef<Path>,
        policy: StoragePolicy,
        options: SqlitePersistenceOptions,
    ) -> AppResult<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    AppError::Infrastructure(format!("create data dir {}: {e}", parent.display()))
                })?;
            }
        }
        let mut conn = Connection::open(&path).map_err(|e| map_sqlite(SqliteOperation::Open, e))?;
        conn.busy_timeout(Duration::from_millis(options.busy_timeout_ms))
            .map_err(|e| map_sqlite(SqliteOperation::BusyTimeout, e))?;
        init_schema(&mut conn)?;
        Ok(Self {
            inner: Arc::new(Inner {
                path,
                policy,
                options,
                conn: std::sync::Mutex::new(conn),
            }),
        })
    }

    fn run_with_busy_retry<T>(
        &self,
        mut op: impl FnMut(&mut Connection) -> Result<T, rusqlite::Error>,
    ) -> AppResult<T> {
        let attempts = self.inner.options.busy_retry_attempts.max(1);
        for attempt in 0..attempts {
            let mut guard = self
                .inner
                .conn
                .lock()
                .map_err(|_| AppError::Infrastructure(SQLITE_MUTEX_POISONED.to_string()))?;
            match op(&mut guard) {
                Ok(v) => return Ok(v),
                Err(e) if is_sqlite_busy(&e) && attempt + 1 < attempts => {
                    drop(guard);
                    std::thread::sleep(Duration::from_millis(
                        BUSY_RETRY_BACKOFF_BASE_MS * u64::from(attempt + 1),
                    ));
                }
                Err(e) => return Err(map_sqlite(SqliteOperation::Exec, e)),
            }
        }
        Err(AppError::Infrastructure(
            SQLITE_BUSY_RETRIES_EXHAUSTED.to_string(),
        ))
    }

    fn db_file_bytes(&self) -> u64 {
        std::fs::metadata(&self.inner.path)
            .map(|m| m.len())
            .unwrap_or(0)
    }

    fn prune_uplinks_sql(&self, conn: &mut Connection) -> rusqlite::Result<()> {
        let max = self.inner.policy.max_records_telemetry;
        let q = schema::sql_count_rows(names::UPLINKS);
        let count: u64 = conn.query_row(q.as_str(), [], |r| r.get(0))?;
        if count <= max {
            return Ok(());
        }
        let excess = count - max;
        let del = schema::sql_prune_uplinks_oldest();
        conn.execute(del.as_str(), params![excess as i64])?;
        Ok(())
    }

    fn prune_audit_sql(&self, conn: &mut Connection) -> rusqlite::Result<()> {
        let max = self.inner.policy.max_records_operational;
        let q = schema::sql_count_rows(names::AUDIT_EVENTS);
        let count: u64 = conn.query_row(q.as_str(), [], |r| r.get(0))?;
        if count <= max {
            return Ok(());
        }
        let excess = count - max;
        let del = schema::sql_prune_audit_oldest();
        conn.execute(del.as_str(), params![excess as i64])?;
        Ok(())
    }

    fn prune_sessions_lru_sql(&self, conn: &mut Connection) -> rusqlite::Result<()> {
        let max = self.inner.policy.max_records_critical;
        let q = schema::sql_count_rows(names::SESSIONS);
        let count: u64 = conn.query_row(q.as_str(), [], |r| r.get(0))?;
        if count <= max {
            return Ok(());
        }
        let excess = count - max;
        let del = schema::sql_prune_sessions_lru();
        conn.execute(del.as_str(), params![excess as i64])?;
        Ok(())
    }

    fn prune_hard_limit_circular_sql(&self, conn: &mut Connection) -> rusqlite::Result<()> {
        if !self.inner.policy.circular_at_hard_limit {
            return Ok(());
        }
        let Some(total) = self.inner.options.total_disk_bytes.filter(|t| *t > 0) else {
            return Ok(());
        };
        let db_bytes = self.db_file_bytes();
        let ratio = db_bytes as f64 / total as f64;
        if ratio < DISK_RATIO_HARD_LIMIT_ENTER {
            return Ok(());
        }
        for _ in 0..HARD_TRIM_MAX_ROUNDS {
            let u = schema::sql_hard_trim_uplinks(HARD_TRIM_UPLINK_BATCH);
            conn.execute(u.as_str(), [])?;
            let a = schema::sql_hard_trim_audit(HARD_TRIM_AUDIT_BATCH);
            conn.execute(a.as_str(), [])?;
            let s = schema::sql_hard_trim_sessions(HARD_TRIM_SESSION_BATCH);
            conn.execute(s.as_str(), [])?;
            let new_ratio = self.db_file_bytes() as f64 / total as f64;
            if new_ratio < DISK_RATIO_HARD_LIMIT_TARGET {
                break;
            }
        }
        Ok(())
    }

    fn counts(conn: &Connection) -> AppResult<(u64, u64, u64)> {
        let qu = schema::sql_count_rows(names::UPLINKS);
        let uplinks: u64 = conn
            .query_row(qu.as_str(), [], |r| r.get(0))
            .map_err(|e| map_sqlite(SqliteOperation::CountUplinks, e))?;
        let qa = schema::sql_count_rows(names::AUDIT_EVENTS);
        let audit: u64 = conn
            .query_row(qa.as_str(), [], |r| r.get(0))
            .map_err(|e| map_sqlite(SqliteOperation::CountAudit, e))?;
        let qs = schema::sql_count_rows(names::SESSIONS);
        let sessions: u64 = conn
            .query_row(qs.as_str(), [], |r| r.get(0))
            .map_err(|e| map_sqlite(SqliteOperation::CountSessions, e))?;
        Ok((uplinks, audit, sessions))
    }

    fn pressure_snapshot_blocking(&self) -> StoragePressureSnapshot {
        let db_bytes = self.db_file_bytes();
        let policy = &self.inner.policy;
        let total = self.inner.options.total_disk_bytes;

        let (uplink_c, audit_c, sess_c) = self
            .inner
            .conn
            .lock()
            .ok()
            .and_then(|g| Self::counts(&g).ok())
            .unwrap_or((0, 0, 0));

        let disk_level = total.and_then(|t| {
            if t == 0 {
                return None;
            }
            let ratio = db_bytes as f64 / t as f64;
            let e = policy.elevated_use_ratio as f64;
            let c = policy.critical_use_ratio as f64;
            Some(if ratio >= DISK_RATIO_HARD_LIMIT_ENTER {
                StoragePressureLevel::HardLimit
            } else if ratio >= c {
                StoragePressureLevel::Critical
            } else if ratio >= e {
                StoragePressureLevel::Elevated
            } else {
                StoragePressureLevel::Normal
            })
        });

        let tier_level = {
            let tel = policy.max_records_telemetry.max(1);
            let op = policy.max_records_operational.max(1);
            let cr = policy.max_records_critical.max(1);
            let ru = uplink_c as f64 / tel as f64;
            let ra = audit_c as f64 / op as f64;
            let rs = sess_c as f64 / cr as f64;
            let r = ru.max(ra).max(rs);
            if r >= TIER_FILL_CRITICAL_RATIO {
                StoragePressureLevel::Critical
            } else if r >= TIER_FILL_ELEVATED_RATIO {
                StoragePressureLevel::Elevated
            } else {
                StoragePressureLevel::Normal
            }
        };

        let level = match (disk_level, tier_level) {
            (Some(d), t) => max_pressure(d, t),
            (None, t) => t,
        };

        let detail = Some(format!(
            "db_bytes={db_bytes} uplinks={uplink_c} audit={audit_c} sessions={sess_c} total_disk={total:?}"
        ));

        StoragePressureSnapshot {
            level,
            db_bytes,
            total_disk_bytes: total,
            detail,
        }
    }

    async fn run_blocking<T: Send + 'static>(
        &self,
        f: impl FnOnce(&SqlitePersistence) -> AppResult<T> + Send + 'static,
    ) -> AppResult<T> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || f(&this))
            .await
            .map_err(|e| AppError::Infrastructure(format!("join blocking task: {e}")))?
    }
}

fn is_sqlite_busy(e: &rusqlite::Error) -> bool {
    matches!(
        e,
        rusqlite::Error::SqliteFailure(ie, _) if ie.code == rusqlite::ErrorCode::DatabaseBusy
    )
}

fn map_sqlite(ctx: SqliteOperation, e: rusqlite::Error) -> AppError {
    AppError::Infrastructure(format!("sqlite {ctx}: {e}"))
}

fn init_schema(conn: &mut Connection) -> AppResult<()> {
    conn.execute_batch(schema::DDL_INIT)
        .map_err(|e| map_sqlite(SqliteOperation::Schema, e))?;
    Ok(())
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis().min(i64::MAX as u128) as i64)
        .unwrap_or(0)
}

fn row_to_session(row: &rusqlite::Row<'_>) -> rusqlite::Result<SessionSnapshot> {
    let dev_addr_i: i64 = row.get(0)?;
    let dev_eui_bytes: Vec<u8> = row.get(1)?;
    let region_s: String = row.get(2)?;
    let class_s: String = row.get(3)?;
    let uplink_fcnt: i64 = row.get(4)?;
    let downlink_fcnt: i64 = row.get(5)?;
    let mut eui_arr = [0u8; DEV_EUI_BYTE_LEN];
    if dev_eui_bytes.len() == DEV_EUI_BYTE_LEN {
        eui_arr.copy_from_slice(&dev_eui_bytes[..DEV_EUI_BYTE_LEN]);
    }
    let region: RegionId = region_s.parse().map_err(|_| {
        rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                STORED_FIELD_REGION,
            )),
        )
    })?;
    let tag = PersistedDeviceClassTag::try_from(class_s.as_str()).map_err(|_| {
        rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                STORED_FIELD_DEVICE_CLASS,
            )),
        )
    })?;
    Ok(SessionSnapshot {
        dev_eui: DevEui(maverick_domain::identifiers::Eui64(eui_arr)),
        dev_addr: DevAddr(dev_addr_i as u32),
        region,
        class: tag.into(),
        uplink_frame_counter: uplink_fcnt as u32,
        downlink_frame_counter: downlink_fcnt as u32,
    })
}

fn max_pressure(a: StoragePressureLevel, b: StoragePressureLevel) -> StoragePressureLevel {
    use StoragePressureLevel::*;
    let rank = |l| match l {
        Normal => 0,
        Elevated => 1,
        Critical => 2,
        HardLimit => 3,
    };
    if rank(a) >= rank(b) {
        a
    } else {
        b
    }
}

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
                let ts = now_ms();
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
                        now_ms(),
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
