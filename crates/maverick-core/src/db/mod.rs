pub mod backend_selector;
pub mod batch_writer;
pub mod database;
pub mod sqlite;

pub use backend_selector::select_database;
pub use batch_writer::BatchWriter;
pub use database::{Database, QueryResult, Row, Value};
pub use sqlite::SqliteDb;

use crate::error::Result;
use std::sync::Arc;

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
        Self {
            db: self.db.clone(),
        }
    }
}

pub async fn create_database<D: Database>(db: D) -> Result<DatabasePool<D>> {
    Ok(DatabasePool::new(db))
}
