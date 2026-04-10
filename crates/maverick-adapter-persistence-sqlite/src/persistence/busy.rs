//! Busy-handler retry loop for writer contention.

use std::time::Duration;

use maverick_core::error::{AppError, AppResult};
use rusqlite::Connection;

use crate::diag::{SQLITE_BUSY_RETRIES_EXHAUSTED, SQLITE_MUTEX_POISONED};
use crate::limits::BUSY_RETRY_BACKOFF_BASE_MS;
use crate::sqlite_op::SqliteOperation;

use super::sql::map_sqlite;
use super::SqlitePersistence;

fn is_sqlite_busy(e: &rusqlite::Error) -> bool {
    matches!(
        e,
        rusqlite::Error::SqliteFailure(ie, _) if ie.code == rusqlite::ErrorCode::DatabaseBusy
    )
}

impl SqlitePersistence {
    pub(crate) fn run_with_busy_retry<T>(
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
}
