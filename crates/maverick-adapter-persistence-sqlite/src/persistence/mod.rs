//! SQLite-backed persistence: composition root for retention and port implementations.

mod busy;
mod pressure;
mod pruning;
mod repos;
mod sql;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use maverick_core::error::{AppError, AppResult};
use maverick_core::storage::StoragePolicy;
use rusqlite::Connection;

use crate::sqlite_op::SqliteOperation;

use sql::{init_schema, map_sqlite};

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
    pub(super) inner: Arc<Inner>,
}

pub(super) struct Inner {
    pub(super) path: PathBuf,
    pub(super) policy: StoragePolicy,
    pub(super) options: SqlitePersistenceOptions,
    pub(super) conn: std::sync::Mutex<Connection>,
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

    fn db_file_bytes(&self) -> u64 {
        std::fs::metadata(&self.inner.path)
            .map(|m| m.len())
            .unwrap_or(0)
    }

    /// Checkpoint the SQLite WAL before process exit (RELI-02).
    ///
    /// Call from main() before std::process::exit to flush all committed WAL frames
    /// to the main database file. rusqlite 0.33 Connection::drop does NOT trigger
    /// WAL checkpoint automatically.
    pub fn close(self) -> AppResult<()> {
        if Arc::strong_count(&self.inner) == 1 {
            let guard = self.inner.conn.lock().map_err(|_| {
                AppError::Infrastructure(
                    "mutex_poisoned: cannot checkpoint on close".to_string(),
                )
            })?;
            guard
                .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
                .map_err(|e| {
                    AppError::Infrastructure(format!("wal_checkpoint on close: {e}"))
                })?;
        }
        Ok(())
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
