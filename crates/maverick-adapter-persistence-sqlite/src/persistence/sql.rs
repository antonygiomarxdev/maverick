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
    })
}
