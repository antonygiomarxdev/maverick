//! SQLite helpers: schema init, row mapping, error labels.

use std::time::{SystemTime, UNIX_EPOCH};

use maverick_core::error::AppError;
use maverick_domain::{DevAddr, DevEui, RegionId, SessionSnapshot};
use rusqlite::Connection;

use crate::diag::{STORED_FIELD_DEVICE_CLASS, STORED_FIELD_REGION};
use crate::limits::DEV_EUI_BYTE_LEN;
use crate::persisted_device_class::PersistedDeviceClassTag;
use crate::schema;
use crate::sqlite_op::SqliteOperation;

/// Wall-clock milliseconds for persisted rows (SQLite `INTEGER` affinity).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct UnixMillis(pub i64);

pub(crate) fn map_sqlite(ctx: SqliteOperation, e: rusqlite::Error) -> AppError {
    AppError::Infrastructure(format!("sqlite {ctx}: {e}"))
}

pub(crate) fn init_schema(conn: &mut Connection) -> Result<(), AppError> {
    conn.execute_batch(schema::DDL_INIT)
        .map_err(|e| map_sqlite(SqliteOperation::Schema, e))?;
    migrate_legacy_columns(conn)?;
    migrate_lns_devices_v2(conn)?;
    migrate_sessions_v2(conn)?;
    migrate_uplinks_v2(conn)?;
    Ok(())
}

/// Best-effort `ALTER TABLE` for databases created before LNS columns existed.
fn migrate_legacy_columns(conn: &mut Connection) -> Result<(), AppError> {
    let _ = conn.execute("ALTER TABLE sessions ADD COLUMN application_id TEXT", []);
    let _ = conn.execute("ALTER TABLE uplinks ADD COLUMN application_id TEXT", []);
    Ok(())
}

/// Recreate `lns_devices` when upgrading from older dev DBs (no `activation_mode` / nullable `dev_addr`).
fn migrate_lns_devices_v2(conn: &mut Connection) -> Result<(), AppError> {
    let has_activation: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('lns_devices') WHERE name = 'activation_mode'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    if has_activation > 0 {
        return Ok(());
    }
    conn.execute_batch(
        r#"
BEGIN IMMEDIATE;
CREATE TABLE lns_devices_migrate_v2 (
    dev_eui BLOB NOT NULL PRIMARY KEY,
    dev_addr INTEGER UNIQUE,
    activation_mode TEXT NOT NULL DEFAULT 'abp',
    application_id TEXT NOT NULL,
    region TEXT NOT NULL,
    enabled INTEGER NOT NULL,
    join_eui BLOB,
    app_key BLOB,
    nwk_key BLOB,
    apps_key BLOB,
    nwks_key BLOB
);
INSERT INTO lns_devices_migrate_v2 (dev_eui, dev_addr, activation_mode, application_id, region, enabled, join_eui, app_key, nwk_key, apps_key, nwks_key)
SELECT dev_eui, dev_addr, 'abp', application_id, region, enabled, join_eui, app_key, nwk_key, NULL, NULL
FROM lns_devices;
DROP TABLE lns_devices;
ALTER TABLE lns_devices_migrate_v2 RENAME TO lns_devices;
CREATE INDEX IF NOT EXISTS idx_lns_devices_dev_addr ON lns_devices(dev_addr);
COMMIT;
"#,
    )
    .map_err(|e| map_sqlite(SqliteOperation::Schema, e))?;
    Ok(())
}

/// Add nwk_s_key and app_s_key columns to sessions (for DBs created before Phase 1).
fn migrate_sessions_v2(conn: &mut Connection) -> Result<(), AppError> {
    let _ = conn.execute("ALTER TABLE sessions ADD COLUMN nwk_s_key BLOB", []);
    let _ = conn.execute("ALTER TABLE sessions ADD COLUMN app_s_key BLOB", []);
    Ok(())
}

/// Add received_at_ms and payload_decrypted to uplinks; add dedup index.
fn migrate_uplinks_v2(conn: &mut Connection) -> Result<(), AppError> {
    let _ = conn.execute("ALTER TABLE uplinks ADD COLUMN received_at_ms INTEGER", []);
    let _ = conn.execute("ALTER TABLE uplinks ADD COLUMN payload_decrypted BLOB", []);
    let _ = conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_uplinks_dedup ON uplinks(dev_addr, f_cnt, received_at_ms)",
        [],
    );
    Ok(())
}

pub(crate) fn now_ms() -> UnixMillis {
    let raw = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis().min(i64::MAX as u128) as i64)
        .unwrap_or(0);
    UnixMillis(raw)
}

pub(crate) fn row_to_session(row: &rusqlite::Row<'_>) -> rusqlite::Result<SessionSnapshot> {
    let dev_addr_i: i64 = row.get(0)?;
    let dev_eui_bytes: Vec<u8> = row.get(1)?;
    let region_s: String = row.get(2)?;
    let class_s: String = row.get(3)?;
    let uplink_fcnt: i64 = row.get(4)?;
    let downlink_fcnt: i64 = row.get(5)?;
    let application_id: Option<String> = row.get(6)?;
    let nwk_s_key_bytes: Vec<u8> = row.get(7).unwrap_or_default();
    let mut nwk_s_key = [0u8; 16];
    if nwk_s_key_bytes.len() == 16 {
        nwk_s_key.copy_from_slice(&nwk_s_key_bytes);
    }
    let app_s_key_bytes: Vec<u8> = row.get(8).unwrap_or_default();
    let mut app_s_key = [0u8; 16];
    if app_s_key_bytes.len() == 16 {
        app_s_key.copy_from_slice(&app_s_key_bytes);
    }
    let mut eui_arr = [0u8; DEV_EUI_BYTE_LEN];
    if dev_eui_bytes.len() == DEV_EUI_BYTE_LEN {
        eui_arr.copy_from_slice(&dev_eui_bytes[..DEV_EUI_BYTE_LEN]);
    }
    let region: RegionId = region_s.parse().map_err(|_| {
        rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                STORED_FIELD_REGION,
            )),
        )
    })?;
    let tag = PersistedDeviceClassTag::try_from(class_s.as_str()).map_err(|_| {
        rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                STORED_FIELD_DEVICE_CLASS,
            )),
        )
    })?;
    Ok(SessionSnapshot {
        dev_eui: DevEui(maverick_domain::identifiers::Eui64(eui_arr)),
        dev_addr: DevAddr(dev_addr_i as u32),
        region,
        class: tag.into(),
        uplink_frame_counter: uplink_fcnt as u32,
        downlink_frame_counter: downlink_fcnt as u32,
        application_id,
        nwk_s_key,
        app_s_key,
    })
}
