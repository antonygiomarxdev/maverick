use std::path::Path;
use std::sync::Arc;

use crate::db::Database;
use crate::error::{AppError, Result};
use crate::storage_profile::StorageProfile;
use async_trait::async_trait;

const CORE_SCHEMA: &str = include_str!("schema.sql");

pub struct SqliteDb {
    conn: Arc<libsql::Connection>,
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
        let db = libsql::Builder::new_local(path)
            .build()
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        let conn = db
            .connect()
            .map_err(|e| AppError::Database(e.to_string()))?;

        let sqlite = Self {
            conn: Arc::new(conn),
        };

        sqlite.execute_batch(profile_pragmas(profile)).await?;
        sqlite.execute_batch(CORE_SCHEMA).await?;

        Ok(sqlite)
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
        let affected_rows = self
            .conn
            .execute(query, ())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(crate::db::QueryResult {
            affected_rows,
            last_insert_id: statement_sets_last_insert_id(query)
                .then(|| self.conn.last_insert_rowid()),
        })
    }

    async fn execute_batch(&self, queries: &str) -> Result<()> {
        self.conn
            .execute_batch(queries)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    async fn query(&self, query: &str) -> Result<Vec<crate::db::Row>> {
        let mut rows = self
            .conn
            .query(query, ())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        let mut result = Vec::new();

        while let Some(row) = rows
            .next()
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
        {
            result.push(convert_row(&row)?);
        }

        Ok(result)
    }
}

fn statement_sets_last_insert_id(query: &str) -> bool {
    let normalized = query.trim_start().to_ascii_lowercase();
    normalized.starts_with("insert") || normalized.starts_with("replace")
}

fn convert_row(row: &libsql::Row) -> Result<crate::db::Row> {
    let mut values = Vec::with_capacity(row.column_count() as usize);

    for index in 0..row.column_count() {
        let value = row
            .get_value(index)
            .map_err(|e| AppError::Database(e.to_string()))?;
        values.push(convert_value(value));
    }

    Ok(crate::db::Row { values })
}

fn convert_value(value: libsql::Value) -> crate::db::Value {
    match value {
        libsql::Value::Null => crate::db::Value::Null,
        libsql::Value::Integer(value) => crate::db::Value::Integer(value),
        libsql::Value::Real(value) => crate::db::Value::Real(value),
        libsql::Value::Text(value) => crate::db::Value::Text(value),
        libsql::Value::Blob(value) => crate::db::Value::Blob(value),
    }
}

impl Clone for SqliteDb {
    fn clone(&self) -> Self {
        Self {
            conn: self.conn.clone(),
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
