use std::sync::Arc;

use axum::{body::{to_bytes, Body}, http::Request};
use maverick_core::{
    adapters::persistence::{SqliteDownlinkRepository, SqliteGatewayRepository},
    api::{create_app, AppState},
    config::RuntimeConfig,
    db::SqliteDb,
    ports::{DownlinkRepository, GatewayRepository},
};
use maverick_domain::{Downlink, DownlinkPriority, Eui64, Frequency, Gateway, GatewayStatus, SpreadingFactor};
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

#[tokio::test]
async fn list_gateways_filters_by_status() {
    let db = SqliteDb::in_memory()
        .await
        .expect("Failed to create in-memory database");
    let repository = SqliteGatewayRepository::new(Arc::new(db.clone()));

    let mut online_gateway = Gateway::new(Eui64::from([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00, 0x11]));
    online_gateway.status = GatewayStatus::Online;
    online_gateway.last_seen = Some(1_700_000_000);
    repository
        .create(online_gateway)
        .await
        .expect("Failed to create online gateway");

    let offline_gateway = Gateway::new(Eui64::from([0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88]));
    repository
        .create(offline_gateway)
        .await
        .expect("Failed to create offline gateway");

    let state = AppState::from_parts(db, RuntimeConfig::default(), "0.1.0-test");
    let app = create_app(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/gateways?status=Online")
                .body(Body::empty())
                .expect("Failed to build request"),
        )
        .await
        .expect("Failed to get response");

    assert_eq!(response.status(), 200);

    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Failed to read response body");
    let payload: serde_json::Value = serde_json::from_slice(&body)
        .expect("Failed to deserialize gateways response");
    let items = payload.as_array().expect("Expected array response");

    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["gateway_eui"], "AABBCCDDEEFF0011");
    assert_eq!(items[0]["status"], "Online");
}

#[tokio::test]
async fn list_healthy_gateways_returns_only_online_gateways() {
    let db = SqliteDb::in_memory()
        .await
        .expect("Failed to create in-memory database");
    let repository = SqliteGatewayRepository::new(Arc::new(db.clone()));

    let mut online_gateway = Gateway::new(Eui64::from([0x10, 0x20, 0x30, 0x40, 0x50, 0x60, 0x70, 0x80]));
    online_gateway.status = GatewayStatus::Online;
    repository
        .create(online_gateway)
        .await
        .expect("Failed to create online gateway");

    let timeout_gateway = Gateway::new(Eui64::from([0x80, 0x70, 0x60, 0x50, 0x40, 0x30, 0x20, 0x10]));
    repository
        .create(timeout_gateway)
        .await
        .expect("Failed to create timeout gateway");

    let state = AppState::from_parts(db, RuntimeConfig::default(), "0.1.0-test");
    let app = create_app(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/gateways/healthy")
                .body(Body::empty())
                .expect("Failed to build request"),
        )
        .await
        .expect("Failed to get response");

    assert_eq!(response.status(), 200);

    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Failed to read response body");
    let payload: serde_json::Value = serde_json::from_slice(&body)
        .expect("Failed to deserialize gateways response");
    let items = payload.as_array().expect("Expected array response");

    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["gateway_eui"], "1020304050607080");
    assert_eq!(items[0]["status"], "Online");
}

#[tokio::test]
async fn list_downlinks_filters_by_state_for_device() {
    let db = SqliteDb::in_memory()
        .await
        .expect("Failed to create in-memory database");
    let repository = SqliteDownlinkRepository::new(Arc::new(db.clone()));

    let dev_eui = Eui64::from([0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88]);
    let gateway_eui = Eui64::from([0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]);
    let downlink = Downlink::new(
        vec![0x01, 0x02],
        10,
        dev_eui,
        gateway_eui,
        Frequency::new(868_100_000),
        SpreadingFactor::new(7).expect("SF must be valid"),
        0,
        1,
    )
    .with_priority(DownlinkPriority::High);
    let sent_id = repository
        .enqueue(downlink)
        .await
        .expect("Failed to enqueue sent downlink");
    repository
        .mark_sent(sent_id, 100)
        .await
        .expect("Failed to mark sent downlink");

    let failed_downlink = Downlink::new(
        vec![0x03, 0x04],
        11,
        dev_eui,
        gateway_eui,
        Frequency::new(868_300_000),
        SpreadingFactor::new(8).expect("SF must be valid"),
        0,
        2,
    );
    let failed_id = repository
        .enqueue(failed_downlink)
        .await
        .expect("Failed to enqueue failed downlink");
    repository
        .mark_failed(failed_id, "gateway timeout")
        .await
        .expect("Failed to mark failed downlink");

    let state = AppState::from_parts(db, RuntimeConfig::default(), "0.1.0-test");
    let app = create_app(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/devices/1122334455667788/downlinks?state=Sent")
                .body(Body::empty())
                .expect("Failed to build request"),
        )
        .await
        .expect("Failed to get response");

    assert_eq!(response.status(), 200);

    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Failed to read response body");
    let payload: serde_json::Value = serde_json::from_slice(&body)
        .expect("Failed to deserialize downlinks response");
    let items = payload.as_array().expect("Expected array response");

    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["downlink_id"], sent_id);
    assert_eq!(items[0]["state"], "Sent");
}

#[tokio::test]
async fn enqueue_downlink_auto_selects_best_healthy_gateway_when_missing() {
    let db = SqliteDb::in_memory()
        .await
        .expect("Failed to create in-memory database");
    let repository = SqliteGatewayRepository::new(Arc::new(db.clone()));

    let mut recent_gateway = Gateway::new(Eui64::from([0xAA, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01]));
    recent_gateway.status = GatewayStatus::Online;
    recent_gateway.last_seen = Some(2_000_000_000);
    recent_gateway.tx_frequency = Some(868_100_000);
    repository
        .create(recent_gateway)
        .await
        .expect("Failed to create recent gateway");

    let mut older_gateway = Gateway::new(Eui64::from([0xAA, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02]));
    older_gateway.status = GatewayStatus::Online;
    older_gateway.last_seen = Some(1_900_000_000);
    repository
        .create(older_gateway)
        .await
        .expect("Failed to create older gateway");

    let state = AppState::from_parts(db, RuntimeConfig::default(), "0.1.0-test");
    let app = create_app(state);

    let enqueue_payload = "{\"payload\":\"AQI=\",\"f_port\":10,\"frequency_hz\":868100000,\"spreading_factor\":7,\"frame_counter\":1,\"priority\":\"High\"}";

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

    let body = to_bytes(get_response.into_body(), usize::MAX)
        .await
        .expect("Failed to read response body");
    let payload: serde_json::Value = serde_json::from_slice(&body)
        .expect("Failed to deserialize downlink response");

    assert_eq!(payload["gateway_eui"], "AA00000000000001");
}

#[tokio::test]
async fn enqueue_downlink_without_gateway_fails_when_no_healthy_gateways_exist() {
    let db = SqliteDb::in_memory()
        .await
        .expect("Failed to create in-memory database");
    let state = AppState::from_parts(db, RuntimeConfig::default(), "0.1.0-test");
    let app = create_app(state);

    let enqueue_payload = "{\"payload\":\"AQI=\",\"f_port\":10,\"frequency_hz\":868100000,\"spreading_factor\":7,\"frame_counter\":1,\"priority\":\"High\"}";

    let response = app
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

    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Failed to read error response body");
    assert_eq!(
        status,
        409,
        "unexpected response body: {}",
        String::from_utf8_lossy(&body)
    );
}
