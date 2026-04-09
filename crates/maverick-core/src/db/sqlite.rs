use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::db::Database;
use crate::error::{AppError, Result};
use crate::storage_profile::StorageProfile;
use async_trait::async_trait;
use rusqlite::types::Value as RusqliteValue;

const CORE_SCHEMA: &str = include_str!("schema.sql");

pub struct SqliteDb {
    conn: Arc<Mutex<rusqlite::Connection>>,
}

impl SqliteDb {
    pub async fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        Self::open_with_profile(path, StorageProfile::Mid).await
    }

    pub async fn new_with_profile<P: AsRef<Path>>(
        path: P,
        profile: StorageProfile,
    ) -> Result<Self> {
        Self::open_with_profile(path, profile).await
    }

    pub async fn in_memory() -> Result<Self> {
        Self::open_with_profile(":memory:", StorageProfile::Mid).await
    }

    pub async fn in_memory_with_profile(profile: StorageProfile) -> Result<Self> {
        Self::open_with_profile(":memory:", profile).await
    }

    async fn open_with_profile<P: AsRef<Path>>(path: P, profile: StorageProfile) -> Result<Self> {
        let path = path.as_ref().to_string_lossy().into_owned();
        let conn = tokio::task::spawn_blocking(move || -> Result<rusqlite::Connection> {
            let conn = if path == ":memory:" {
                rusqlite::Connection::open_in_memory()
            } else {
                rusqlite::Connection::open(&path)
            };
            conn.map_err(|e| AppError::Database(e.to_string()))
        })
        .await
        .map_err(|e| AppError::Database(e.to_string()))??;

        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };

        db.execute_batch(profile_pragmas(profile)).await?;
        db.execute_batch(CORE_SCHEMA).await?;

        Ok(db)
    }
}

fn profile_pragmas(profile: StorageProfile) -> &'static str {
    match profile {
        StorageProfile::High => {
            "
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA temp_store = MEMORY;
            PRAGMA cache_size = -20000;
            PRAGMA wal_autocheckpoint = 1000;
            "
        }
        StorageProfile::Mid | StorageProfile::Auto => {
            "
            PRAGMA journal_mode = DELETE;
            PRAGMA synchronous = NORMAL;
            PRAGMA temp_store = MEMORY;
            PRAGMA cache_size = -8000;
            "
        }
        StorageProfile::Extreme => {
            "
            PRAGMA journal_mode = MEMORY;
            PRAGMA synchronous = OFF;
            PRAGMA temp_store = MEMORY;
            PRAGMA cache_size = -2048;
            "
        }
    }
}

#[async_trait]
impl Database for SqliteDb {
    async fn execute(&self, query: &str) -> Result<crate::db::QueryResult> {
        let conn = Arc::clone(&self.conn);
        let query = query.to_string();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().unwrap();
            let affected = conn.execute(&query, []).map_err(map_rusqlite_error)?;
            let last_insert_id =
                statement_sets_last_insert_id(&query).then(|| conn.last_insert_rowid());
            Ok(crate::db::QueryResult {
                affected_rows: affected as u64,
                last_insert_id,
            })
        })
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
    }

    async fn execute_batch(&self, queries: &str) -> Result<()> {
        let conn = Arc::clone(&self.conn);
        let queries = queries.to_string();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().unwrap();
            conn.execute_batch(&queries)
                .map_err(|e| AppError::Database(e.to_string()))
        })
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
    }

    async fn query(&self, query: &str) -> Result<Vec<crate::db::Row>> {
        let conn = Arc::clone(&self.conn);
        let query = query.to_string();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().unwrap();
            let mut stmt = conn
                .prepare(&query)
                .map_err(|e| AppError::Database(e.to_string()))?;
            let col_count = stmt.column_count();
            let rows = stmt
                .query_map([], |row| {
                    let mut values = Vec::with_capacity(col_count);
                    for i in 0..col_count {
                        let value: RusqliteValue = row.get(i)?;
                        values.push(convert_value(value));
                    }
                    Ok(crate::db::Row { values })
                })
                .map_err(|e| AppError::Database(e.to_string()))?
                .collect::<std::result::Result<Vec<_>, rusqlite::Error>>()
                .map_err(|e| AppError::Database(e.to_string()))?;
            Ok(rows)
        })
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
    }
}

fn statement_sets_last_insert_id(query: &str) -> bool {
    let normalized = query.trim_start().to_ascii_lowercase();
    normalized.starts_with("insert") || normalized.starts_with("replace")
}

fn map_rusqlite_error(e: rusqlite::Error) -> AppError {
    match &e {
        rusqlite::Error::SqliteFailure(err, _)
            if err.code == rusqlite::ErrorCode::ConstraintViolation =>
        {
            AppError::ConstraintViolation(e.to_string())
        }
        _ => AppError::Database(e.to_string()),
    }
}

fn convert_value(value: RusqliteValue) -> crate::db::Value {
    match value {
        RusqliteValue::Null => crate::db::Value::Null,
        RusqliteValue::Integer(i) => crate::db::Value::Integer(i),
        RusqliteValue::Real(f) => crate::db::Value::Real(f),
        RusqliteValue::Text(s) => crate::db::Value::Text(s),
        RusqliteValue::Blob(b) => crate::db::Value::Blob(b),
    }
}

impl Clone for SqliteDb {
    fn clone(&self) -> Self {
        Self {
            conn: Arc::clone(&self.conn),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SqliteDb;
    use crate::db::{Database, Value};

    #[tokio::test]
    async fn execute_reports_last_insert_id_for_insert_statements() {
        let db = SqliteDb::in_memory()
            .await
            .expect("in-memory sqlite must open");

        db.execute_batch(
            "CREATE TABLE device_test_records (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL);",
        )
        .await
        .expect("schema must be created");

        let result = db
            .execute("INSERT INTO device_test_records (name) VALUES ('sensor-a')")
            .await
            .expect("insert must succeed");

        assert_eq!(result.affected_rows, 1);
        assert_eq!(result.last_insert_id, Some(1));
    }

    #[tokio::test]
    async fn query_maps_sqlite_values_to_database_values() {
        let db = SqliteDb::in_memory()
            .await
            .expect("in-memory sqlite must open");

        db.execute_batch(
            "
            CREATE TABLE telemetry (
                id INTEGER PRIMARY KEY,
                label TEXT NOT NULL,
                battery REAL NOT NULL,
                payload BLOB NOT NULL,
                notes TEXT
            );
            INSERT INTO telemetry (id, label, battery, payload, notes)
            VALUES (7, 'node-1', 87.5, X'010203', NULL);
            ",
        )
        .await
        .expect("seed data must be inserted");

        let rows = db
            .query("SELECT id, label, battery, payload, notes FROM telemetry")
            .await
            .expect("query must succeed");

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].values.len(), 5);
        assert!(matches!(rows[0].values[0], Value::Integer(7)));
        assert!(matches!(rows[0].values[1], Value::Text(ref value) if value == "node-1"));
        assert!(
            matches!(rows[0].values[2], Value::Real(value) if (value - 87.5).abs() < f64::EPSILON)
        );
        assert!(matches!(rows[0].values[3], Value::Blob(ref value) if value == &vec![1, 2, 3]));
        assert!(matches!(rows[0].values[4], Value::Null));
    }
}
