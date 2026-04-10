//! Retention pruning and hard-limit circular trim (SQL only; policy from core).

use rusqlite::{params, Connection};

use crate::limits::{
    DISK_RATIO_HARD_LIMIT_ENTER, DISK_RATIO_HARD_LIMIT_TARGET, HARD_TRIM_AUDIT_BATCH,
    HARD_TRIM_MAX_ROUNDS, HARD_TRIM_SESSION_BATCH, HARD_TRIM_UPLINK_BATCH,
};
use crate::schema::{self, names};

use super::SqlitePersistence;

impl SqlitePersistence {
    pub(crate) fn prune_uplinks_sql(&self, conn: &mut Connection) -> rusqlite::Result<()> {
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

    pub(crate) fn prune_audit_sql(&self, conn: &mut Connection) -> rusqlite::Result<()> {
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

    pub(crate) fn prune_sessions_lru_sql(&self, conn: &mut Connection) -> rusqlite::Result<()> {
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

    pub(crate) fn prune_hard_limit_circular_sql(
        &self,
        conn: &mut Connection,
    ) -> rusqlite::Result<()> {
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
}
