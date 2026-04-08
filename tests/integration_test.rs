use maverick_core::{api::{create_app, AppState}, db::DbPool};
use axum::{body::Body, http::Request};
use tower::ServiceExt;

#[tokio::test]
async fn health_endpoint_returns_ok() {
    let db = DbPool::in_memory().expect("Failed to create in-memory database");
    let state = AppState::new(db, "0.1.0-test");
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