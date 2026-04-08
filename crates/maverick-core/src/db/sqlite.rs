use std::path::Path;
use std::sync::Arc;

use crate::error::{AppError, Result};
use crate::db::Database;
use async_trait::async_trait;

pub struct SqliteDb {
    conn: Arc<libsql::Connection>,
}

impl SqliteDb {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_str = path.as_ref().to_string_lossy().to_string();
        let db = libsql::Database::open(&path_str).map_err(|e| AppError::Database(e.to_string()))?;
        let conn = db.connect().map_err(|e| AppError::Database(e.to_string()))?;
        Ok(Self { conn: Arc::new(conn) })
    }

    pub fn in_memory() -> Result<Self> {
        let db = libsql::Database::open(":memory:").map_err(|e| AppError::Database(e.to_string()))?;
        let conn = db.connect().map_err(|e| AppError::Database(e.to_string()))?;
        Ok(Self { conn: Arc::new(conn) })
    }
}

#[async_trait]
impl Database for SqliteDb {
    async fn execute(&self, query: &str) -> Result<crate::db::QueryResult> {
        let params: Vec<String> = Vec::new();
        let affected_rows = self.conn.execute(query, params).await.map_err(|e| AppError::Database(e.to_string()))?;
        Ok(crate::db::QueryResult {
            affected_rows,
            last_insert_id: None,
        })
    }

    async fn execute_batch(&self, queries: &str) -> Result<()> {
        self.conn.execute_batch(queries).await.map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    async fn query(&self, _query: &str) -> Result<Vec<crate::db::Row>> {
        Ok(Vec::new())
    }
}

impl Clone for SqliteDb {
    fn clone(&self) -> Self {
        Self {
            conn: self.conn.clone(),
        }
    }
}