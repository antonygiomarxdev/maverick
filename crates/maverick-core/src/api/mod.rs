pub mod routes;

use axum::Router;
use std::sync::Arc;

use crate::db::Database;

#[derive(Clone)]
pub struct AppState<D: Database> {
    pub db: Arc<D>,
    pub version: &'static str,
}

impl<D: Database> AppState<D> {
    pub fn new(db: D, version: &'static str) -> Self {
        Self {
            db: Arc::new(db),
            version,
        }
    }
}

pub fn create_app<D: Database + Clone + Send + Sync + 'static>(state: AppState<D>) -> Router {
    Router::new()
        .nest("/api/v1", routes::routes())
        .with_state(state)
}