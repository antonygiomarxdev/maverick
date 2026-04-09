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

#[tokio::test]
async fn downlink_enqueue_and_get_returns_expected_status_codes() {
    let db = SqliteDb::in_memory()
        .await
        .expect("Failed to create in-memory database");
    let state = AppState::from_parts(db, RuntimeConfig::default(), "0.1.0-test");
    let app = create_app(state);

    let enqueue_payload = "{\"gateway_eui\":\"0102030405060708\",\"payload\":\"AQI=\",\"f_port\":10,\"frequency_hz\":868100000,\"spreading_factor\":7,\"frame_counter\":1,\"priority\":\"High\"}";

    let enqueue_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/devices/1122334455667788/downlinks")
                .header("content-type", "application/json")
                .body(Body::from(enqueue_payload))
                .expect("Failed to build request"),
        )
        .await
        .expect("Failed to enqueue downlink");

    assert_eq!(enqueue_response.status(), 202);

    let get_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/devices/1122334455667788/downlinks/1")
                .body(Body::empty())
                .expect("Failed to build request"),
        )
        .await
        .expect("Failed to get downlink");

    assert_eq!(get_response.status(), 200);
}
