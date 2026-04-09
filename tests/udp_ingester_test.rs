use std::sync::Arc;

use maverick_core::adapters::persistence::{SqliteAuditLogWriter, SqliteGatewayRepository, SqliteUplinkRepository};
use maverick_core::db::{Database, SqliteDb};
use maverick_core::events::EventBus;
use maverick_core::ingester::semtech::parse_push_data;
use maverick_core::use_cases::IngestUplinkService;

#[tokio::test]
async fn semtech_push_data_ingest_persists_gateway_and_uplink() {
    let json = r#"{"rxpk":[{"tmst":123456,"freq":868.1,"chan":1,"stat":1,"modu":"LORA","datr":"SF7BW125","codr":"4/5","rssi":-45,"lsnr":7.0,"data":"AQID"}]}"#;
    let mut datagram = vec![0x02, 0xAA, 0xBB, 0x00];
    datagram.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
    datagram.extend_from_slice(json.as_bytes());

    let parsed = parse_push_data(&datagram).expect("datagram must parse");
    let db = Arc::new(SqliteDb::in_memory().await.expect("db must open"));
    let service = IngestUplinkService::new(
        SqliteUplinkRepository::new(db.clone()),
        SqliteGatewayRepository::new(db.clone()),
        SqliteAuditLogWriter::new(db.clone()),
        EventBus::new(16),
    );

    for command in parsed.commands {
        service.ingest(command).await.expect("ingest must succeed");
    }

    let gateways = db
        .query("SELECT gateway_eui, status FROM gateways")
        .await
        .expect("query must succeed");
    let uplinks = db
        .query("SELECT gateway_eui, payload FROM uplinks")
        .await
        .expect("query must succeed");

    assert_eq!(gateways.len(), 1);
    assert_eq!(uplinks.len(), 1);
}

#[test]
fn semtech_push_data_rejects_invalid_payload() {
    let json = r#"{"rxpk":[{"tmst":123456,"freq":868.1,"chan":1,"stat":1,"modu":"LORA","datr":"SF7BW125","codr":"4/5","rssi":-45,"lsnr":7.0,"data":"!!!"}]}"#;
    let mut datagram = vec![0x02, 0xAA, 0xBB, 0x00];
    datagram.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
    datagram.extend_from_slice(json.as_bytes());

    let error = parse_push_data(&datagram).expect_err("invalid payload must fail");
    assert!(error.to_string().contains("invalid base64 payload"));
}