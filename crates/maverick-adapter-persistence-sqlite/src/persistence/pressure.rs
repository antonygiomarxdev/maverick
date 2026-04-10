//! Storage pressure snapshot from on-disk size, tier fill, and optional disk hint.

use maverick_core::error::AppResult;
use maverick_core::storage::{StoragePressureLevel, StoragePressureSnapshot};
use rusqlite::Connection;

use crate::limits::{
    DISK_RATIO_HARD_LIMIT_ENTER, TIER_FILL_CRITICAL_RATIO, TIER_FILL_ELEVATED_RATIO,
};
use crate::schema::{self, names};
use crate::sqlite_op::SqliteOperation;

use super::sql::map_sqlite;
use super::SqlitePersistence;

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

impl SqlitePersistence {
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

    pub(crate) fn pressure_snapshot_blocking(&self) -> StoragePressureSnapshot {
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
}
