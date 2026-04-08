use axum::Router;
use axum::http::StatusCode;

use crate::db::Database;
use super::AppState;

fn router<D: Database + Clone + Send + Sync + 'static>() -> Router<AppState<D>> {
    Router::new()
        .route("/health", axum::routing::get(health_check))
}

pub fn routes<D: Database + Clone + Send + Sync + 'static>() -> Router<AppState<D>> {
    router()
}

#[derive(serde::Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
    database: &'static str,
}

async fn health_check<D: Database + Clone + Send + Sync + 'static>(
    state: axum::extract::State<AppState<D>>,
) -> (StatusCode, axum::Json<HealthResponse>) {
    let db_status = match state.db.execute("SELECT 1").await {
        Ok(_) => "healthy",
        Err(_) => "unhealthy",
    };

    let response = HealthResponse {
        status: "ok",
        version: state.version,
        database: db_status,
    };

    (StatusCode::OK, axum::Json(response))
}