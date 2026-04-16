use std::sync::Arc;

use maverick_adapter_persistence_sqlite::{SqlitePersistence, SqlitePersistenceOptions};
use maverick_adapter_radio_udp::parse_push_data_json;
use maverick_core::ports::SessionRepository;
use maverick_core::protocol::LoRaWAN10xClassA;
use maverick_core::storage::StoragePressureSource;
use maverick_core::use_cases::{build_b0_uplink, compute_mic, IngestUplink};
use maverick_core::InstallProfile;
use maverick_domain::identifiers::Eui64;
use maverick_domain::{DevAddr, DevEui, DeviceClass, GatewayEui, RegionId, SessionSnapshot};

#[tokio::test]
async fn operator_local_gateway_flow_ingests_and_persists_uplink() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db = dir.path().join("maverick.db");
    let policy = InstallProfile::Balanced.default_storage_policy();
    let store =
        SqlitePersistence::open(&db, policy, SqlitePersistenceOptions::default()).expect("open");

    let session = SessionSnapshot {
        dev_eui: DevEui(Eui64([0x11; 8])),
        dev_addr: DevAddr(0x0403_0201),
        region: RegionId::Eu868,
        class: DeviceClass::ClassA,
        uplink_frame_counter: 0,
        downlink_frame_counter: 0,
        application_id: None,
        nwk_s_key: [0u8; 16],
        app_s_key: [0u8; 16],
    };
    SessionRepository::upsert(&store, &session)
        .await
        .expect("upsert session");

    let svc = IngestUplink {
        sessions: Arc::new(store.clone()),
        uplinks: Arc::new(store.clone()),
        audit: Arc::new(store.clone()),
        protocol: Arc::new(LoRaWAN10xClassA),
    };

    let gw = GatewayEui(Eui64([1, 2, 3, 4, 5, 6, 7, 8]));
    let gwmp_json = r#"{
      "rxpk":[
        {"freq":868.1,"rssi":-57,"lsnr":5.2,"data":"QAECAwQEAAEByv66vg=="}
      ]
    }"#;
    let batch = parse_push_data_json(gw, 2, gwmp_json).expect("parse gwmp");
    assert_eq!(batch.observations.len(), 1);
    let mut obs = batch.observations.into_iter().next().expect("obs");
    // Compute valid MIC using test session's zero NwkSKey.
    // session.uplink_frame_counter = 0, so reconstructed_fcnt = u32::from(obs.f_cnt).
    let reconstructed_fcnt = u32::from(obs.f_cnt);
    let b0 = build_b0_uplink(obs.dev_addr.0, reconstructed_fcnt, obs.phy_without_mic.len());
    obs.wire_mic = compute_mic(&session.nwk_s_key, &b0, &obs.phy_without_mic);
    svc.execute(obs).await.expect("ingest parsed observation");

    let persisted = SessionRepository::get_by_dev_addr(&store, DevAddr(0x0403_0201))
        .await
        .expect("get session")
        .expect("session exists");
    assert_eq!(persisted.uplink_frame_counter, 256);

    let pressure = StoragePressureSource::pressure_snapshot(&store).await;
    assert!(pressure.db_bytes > 0);
}
