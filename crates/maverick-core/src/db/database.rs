use crate::error::Result;
use async_trait::async_trait;

#[async_trait]
pub trait Database: Send + Sync {
    async fn execute(&self, query: &str) -> Result<QueryResult>;
    async fn execute_batch(&self, queries: &str) -> Result<()>;
    async fn query(&self, query: &str) -> Result<Vec<Row>>;
}

#[derive(Debug, Clone)]
pub struct QueryResult {
    pub affected_rows: u64,
    pub last_insert_id: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct Row {
    pub values: Vec<Value>,
}

#[derive(Debug, Clone)]
pub enum Value {
    Null,
    Integer(i64),
    Real(f64),
    Text(String),
    Blob(Vec<u8>),
}