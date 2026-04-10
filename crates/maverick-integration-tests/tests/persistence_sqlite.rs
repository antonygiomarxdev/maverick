use std::sync::{Arc, Barrier};
use std::time::Duration;

use maverick_adapter_persistence_sqlite::{SqlitePersistence, SqlitePersistenceOptions};
use maverick_core::ports::{SessionRepository, UplinkObservation, UplinkRecord, UplinkRepository};
use maverick_core::protocol::LoRaWAN10xClassA;
use maverick_core::storage::StoragePressureSource;
use maverick_core::use_cases::IngestUplink;
use maverick_core::InstallProfile;
use maverick_domain::identifiers::Eui64;
use maverick_domain::{DevAddr, DevEui, DeviceClass, GatewayEui, RegionId, SessionSnapshot};
use rusqlite::Connection;

#[test]
fn sqlite_ddl_defines_tables_matching_schema_names() {
    use maverick_adapter_persistence_sqlite::schema::{names, DDL_INIT};
    for t in [names::SESSIONS, names::UPLINKS, names::AUDIT_EVENTS] {
        assert!(DDL_INIT.contains(t), "schema.sql must define table {t}");
    }
}

#[tokio::test]
async fn ingest_uplink_persists_via_sqlite_adapter() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db = dir.path().join("maverick.db");
    let policy = InstallProfile::Balanced.default_storage_policy();
    let store =
        SqlitePersistence::open(&db, policy, SqlitePersistenceOptions::default()).expect("open");

    let session = SessionSnapshot {
        dev_eui: DevEui(Eui64([0x11; 8])),
        dev_addr: DevAddr(0xA1_B2_C3_D4),
        region: RegionId::Eu868,
        class: DeviceClass::ClassA,
        uplink_frame_counter: 0,
        downlink_frame_counter: 0,
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

    let obs = UplinkObservation {
        gateway_eui: GatewayEui(Eui64([0x22; 8])),
        dev_addr: DevAddr(0xA1_B2_C3_D4),
        region: RegionId::Eu868,
        f_cnt: 1,
        f_port: 1,
        payload: vec![0x01, 0x02],
        rssi: None,
        snr: None,
    };
    svc.execute(obs).await.expect("ingest");

    let reopened = SqlitePersistence::open(
        &db,
        InstallProfile::Balanced.default_storage_policy(),
        SqlitePersistenceOptions::default(),
    )
    .expect("reopen");
    let s = SessionRepository::get_by_dev_addr(&reopened, DevAddr(0xA1_B2_C3_D4))
        .await
        .expect("get")
        .expect("session exists");
    assert_eq!(s.uplink_frame_counter, 1);
}

fn sample_session(dev_addr: u32) -> SessionSnapshot {
    SessionSnapshot {
        dev_eui: DevEui(Eui64([2u8; 8])),
        dev_addr: DevAddr(dev_addr),
        region: RegionId::Eu868,
        class: DeviceClass::ClassA,
        uplink_frame_counter: 0,
        downlink_frame_counter: 0,
    }
}

#[tokio::test]
async fn sqlite_recovery_after_reopen_preserves_session_and_uplink() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("edge.db");
    let policy = InstallProfile::Balanced.default_storage_policy();
    {
        let p = SqlitePersistence::open(&db, policy.clone(), SqlitePersistenceOptions::default())
            .unwrap();
        SessionRepository::upsert(&p, &sample_session(0x01_02_03_04))
            .await
            .unwrap();
        UplinkRepository::append(
            &p,
            &UplinkRecord {
                dev_addr: DevAddr(0x01_02_03_04),
                f_cnt: 1,
                payload: vec![0xAB],
            },
        )
        .await
        .unwrap();
    }
    let p2 = SqlitePersistence::open(&db, policy, SqlitePersistenceOptions::default()).unwrap();
    let s = SessionRepository::get_by_dev_addr(&p2, DevAddr(0x01_02_03_04))
        .await
        .unwrap()
        .expect("session");
    assert_eq!(s.dev_addr.0, 0x01_02_03_04);
    let snap = StoragePressureSource::pressure_snapshot(&p2).await;
    assert!(snap.db_bytes > 0);
}

#[tokio::test]
async fn sqlite_telemetry_retention_drops_oldest_uplinks() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("edge.db");
    let mut policy = InstallProfile::Constrained.default_storage_policy();
    policy.max_records_telemetry = 3;
    let p = SqlitePersistence::open(&db, policy, SqlitePersistenceOptions::default()).unwrap();
    for i in 0..10 {
        UplinkRepository::append(
            &p,
            &UplinkRecord {
                dev_addr: DevAddr(0x01_02_03_04),
                f_cnt: i,
                payload: vec![i as u8],
            },
        )
        .await
        .unwrap();
    }
    let c = Connection::open(&db).unwrap();
    let count: u64 = c
        .query_row("SELECT COUNT(*) FROM uplinks", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 3);
}

#[tokio::test]
async fn sqlite_concurrent_transaction_waits_on_busy_then_succeeds() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("edge.db");
    let policy = InstallProfile::Balanced.default_storage_policy();
    let p = SqlitePersistence::open(&db_path, policy, SqlitePersistenceOptions::default()).unwrap();

    let path_clone = db_path.clone();
    let barrier = Arc::new(Barrier::new(2));
    let b2 = barrier.clone();
    let t = std::thread::spawn(move || {
        let c = Connection::open(&path_clone).expect("open");
        c.busy_timeout(Duration::from_secs(1)).ok();
        c.execute_batch("BEGIN IMMEDIATE;").expect("begin");
        b2.wait();
        std::thread::sleep(Duration::from_millis(150));
        c.execute_batch("COMMIT;").ok();
    });

    barrier.wait();
    let rec = UplinkRecord {
        dev_addr: DevAddr(1),
        f_cnt: 1,
        payload: vec![1],
    };
    let res = UplinkRepository::append(&p, &rec).await;
    t.join().expect("join");
    res.expect("append should wait on busy lock");
}
