pub mod sqlite;
pub mod database;

pub use database::{Database, QueryResult, Row, Value};
pub use sqlite::SqliteDb;

use std::sync::Arc;
use crate::error::Result;

pub struct DatabasePool<D: Database> {
    db: Arc<D>,
}

impl<D: Database> DatabasePool<D> {
    pub fn new(db: D) -> Self {
        Self { db: Arc::new(db) }
    }

    pub fn db(&self) -> Arc<D> {
        self.db.clone()
    }
}

impl<D: Database> Clone for DatabasePool<D> {
    fn clone(&self) -> Self {
        Self { db: self.db.clone() }
    }
}

pub async fn create_database<D: Database>(db: D) -> Result<DatabasePool<D>> {
    Ok(DatabasePool::new(db))
}