use axum::{body::Body, http::Request};
use tower::ServiceExt;

use maverick_core::{api::{AppState, create_app}, config::RuntimeConfig, db::SqliteDb};

#[tokio::test]
async fn devices_crud_flow_compiles_and_returns_expected_status_codes() {
    let db = SqliteDb::in_memory().await.expect("db must open");
    let state = AppState::from_parts(db, RuntimeConfig::default(), "0.1.0-test");
    let app = create_app(state);

    let create_payload = r#"{
        \"dev_eui\": \"0102030405060708\",
        \"app_eui\": \"0807060504030201\",
        \"app_key\": \"AQEBAQEBAQEBAQEBAQEBAQ==\",
        \"nwk_key\": \"AgICAgICAgICAgICAgICAg==\",
        \"class\": \"ClassA\"
    }"#;

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/devices")
                .header("content-type", "application/json")
                .body(Body::from(create_payload))
                .expect("request must build"),
        )
        .await
        .expect("request must succeed");
    assert_eq!(create_response.status(), 201);

    let get_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/devices/0102030405060708")
                .body(Body::empty())
                .expect("request must build"),
        )
        .await
        .expect("request must succeed");
    assert_eq!(get_response.status(), 200);

    let patch_payload = r#"{ \"class\": \"ClassC\", \"state\": \"JoinPending\" }"#;
    let patch_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/api/v1/devices/0102030405060708")
                .header("content-type", "application/json")
                .body(Body::from(patch_payload))
                .expect("request must build"),
        )
        .await
        .expect("request must succeed");
    assert_eq!(patch_response.status(), 200);

    let delete_response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/v1/devices/0102030405060708")
                .body(Body::empty())
                .expect("request must build"),
        )
        .await
        .expect("request must succeed");
    assert_eq!(delete_response.status(), 204);
}

#[tokio::test]
async fn duplicate_device_create_returns_conflict() {
    let db = SqliteDb::in_memory().await.expect("db must open");
    let state = AppState::from_parts(db, RuntimeConfig::default(), "0.1.0-test");
    let app = create_app(state);
    let payload = r#"{
        \"dev_eui\": \"1111111111111111\",
        \"app_eui\": \"2222222222222222\",
        \"app_key\": \"AQEBAQEBAQEBAQEBAQEBAQ==\",
        \"nwk_key\": \"AgICAgICAgICAgICAgICAg==\"
    }"#;

    let first = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/devices")
                .header("content-type", "application/json")
                .body(Body::from(payload))
                .expect("request must build"),
        )
        .await
        .expect("request must succeed");
    assert_eq!(first.status(), 201);

    let second = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/devices")
                .header("content-type", "application/json")
                .body(Body::from(payload))
                .expect("request must build"),
        )
        .await
        .expect("request must succeed");
    assert_eq!(second.status(), 409);
}

#[tokio::test]
async fn invalid_patch_returns_bad_request() {
    let db = SqliteDb::in_memory().await.expect("db must open");
    let state = AppState::from_parts(db, RuntimeConfig::default(), "0.1.0-test");
    let app = create_app(state);
    let response = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/api/v1/devices/0102030405060708")
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .expect("request must build"),
        )
        .await
        .expect("request must succeed");
    assert_eq!(response.status(), 400);
}

#[tokio::test]
async fn get_missing_device_returns_not_found() {
    let db = SqliteDb::in_memory().await.expect("db must open");
    let state = AppState::from_parts(db, RuntimeConfig::default(), "0.1.0-test");
    let app = create_app(state);
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/devices/AAAAAAAAAAAAAAAA")
                .body(Body::empty())
                .expect("request must build"),
        )
        .await
        .expect("request must succeed");
    assert_eq!(response.status(), 404);
}