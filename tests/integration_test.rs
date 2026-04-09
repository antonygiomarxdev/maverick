use axum::{body::Body, http::Request};
use maverick_core::{
    api::{create_app, AppState},
    config::RuntimeConfig,
    db::SqliteDb,
};
use tower::ServiceExt;

#[tokio::test]
async fn health_endpoint_returns_ok() {
    let db = SqliteDb::in_memory()
        .await
        .expect("Failed to create in-memory database");
    let state = AppState::from_parts(db, RuntimeConfig::default(), "0.1.0-test");
    let app = create_app(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/health")
                .body(Body::empty())
                .expect("Failed to build request"),
        )
        .await
        .expect("Failed to get response");

    assert_eq!(response.status(), 200);
}
