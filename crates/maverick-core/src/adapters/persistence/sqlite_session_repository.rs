use std::sync::Arc;

use async_trait::async_trait;
use maverick_domain::{DeviceSession, Eui64, FrameCounter};

use crate::adapters::persistence::sqlite_utils::{
    blob_literal, optional_i64, required_blob, required_i64,
};
use crate::db::{Database, Row};
use crate::error::Result;
use crate::ports::SessionRepository;

#[derive(Clone)]
pub struct SqliteSessionRepository<D: Database> {
    db: Arc<D>,
}

impl<D: Database> SqliteSessionRepository<D> {
    pub fn new(db: Arc<D>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl<D: Database> SessionRepository for SqliteSessionRepository<D> {
    async fn upsert_for_device(&self, dev_eui: Eui64, session: DeviceSession) -> Result<()> {
        let query = format!(
            "INSERT INTO device_sessions (dev_eui, dev_addr, app_s_key, nwk_s_key, frame_counter, last_join_time, updated_at) VALUES ({}, {}, {}, {}, {}, {}, unixepoch()) ON CONFLICT(dev_eui) DO UPDATE SET dev_addr=excluded.dev_addr, app_s_key=excluded.app_s_key, nwk_s_key=excluded.nwk_s_key, frame_counter=excluded.frame_counter, last_join_time=excluded.last_join_time, updated_at=unixepoch()",
            blob_literal(dev_eui.as_bytes_slice()),
            session.dev_addr,
            blob_literal(&session.app_s_key),
            blob_literal(&session.nwk_s_key),
            session.frame_counter.0,
            optional_i64(session.last_join_time),
        );

        self.db.execute(&query).await?;
        Ok(())
    }

    async fn get_by_dev_eui(&self, dev_eui: Eui64) -> Result<Option<DeviceSession>> {
        let query = format!(
            "SELECT dev_addr, app_s_key, nwk_s_key, frame_counter, last_join_time FROM device_sessions WHERE dev_eui = {} LIMIT 1",
            blob_literal(dev_eui.as_bytes_slice())
        );
        let rows = self.db.query(&query).await?;

        rows.into_iter().next().map(session_from_row).transpose()
    }

    async fn get_by_dev_addr(&self, dev_addr: u32) -> Result<Option<(Eui64, DeviceSession)>> {
        let query = format!(
            "SELECT dev_eui, dev_addr, app_s_key, nwk_s_key, frame_counter, last_join_time FROM device_sessions WHERE dev_addr = {} LIMIT 1",
            dev_addr
        );
        let rows = self.db.query(&query).await?;

        rows.into_iter()
            .next()
            .map(session_and_device_from_row)
            .transpose()
    }
}

fn session_from_row(row: Row) -> Result<DeviceSession> {
    Ok(DeviceSession {
        dev_addr: required_i64(&row, 0, "dev_addr")? as u32,
        app_s_key: required_blob::<16>(&row, 1, "app_s_key")?,
        nwk_s_key: required_blob::<16>(&row, 2, "nwk_s_key")?,
        frame_counter: FrameCounter(required_i64(&row, 3, "frame_counter")? as u32),
        last_join_time: optional_i64_value(&row, 4),
    })
}

fn session_and_device_from_row(row: Row) -> Result<(Eui64, DeviceSession)> {
    let dev_eui = Eui64::from(required_blob::<8>(&row, 0, "dev_eui")?);
    let session = DeviceSession {
        dev_addr: required_i64(&row, 1, "dev_addr")? as u32,
        app_s_key: required_blob::<16>(&row, 2, "app_s_key")?,
        nwk_s_key: required_blob::<16>(&row, 3, "nwk_s_key")?,
        frame_counter: FrameCounter(required_i64(&row, 4, "frame_counter")? as u32),
        last_join_time: optional_i64_value(&row, 5),
    };

    Ok((dev_eui, session))
}

fn optional_i64_value(row: &Row, index: usize) -> Option<i64> {
    match row.values.get(index) {
        Some(crate::db::Value::Integer(value)) => Some(*value),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use maverick_domain::{DeviceSession, Eui64};

    use crate::adapters::persistence::SqliteSessionRepository;
    use crate::db::SqliteDb;
    use crate::ports::SessionRepository;

    #[tokio::test]
    async fn sqlite_session_repository_round_trip() {
        let db = Arc::new(SqliteDb::in_memory().await.expect("db must open"));
        let repository = SqliteSessionRepository::new(db);
        let dev_eui = Eui64::from([1, 2, 3, 4, 5, 6, 7, 8]);
        let mut session = DeviceSession::new(0x26011BDA, [0xAA; 16], [0xBB; 16]);
        session.last_join_time = Some(123456);

        repository
            .upsert_for_device(dev_eui, session.clone())
            .await
            .expect("upsert must succeed");

        let fetched = repository
            .get_by_dev_addr(0x26011BDA)
            .await
            .expect("query must succeed")
            .expect("session must exist");

        assert_eq!(fetched.0.as_bytes(), dev_eui.as_bytes());
        assert_eq!(fetched.1.dev_addr, session.dev_addr);
        assert_eq!(fetched.1.app_s_key, session.app_s_key);
        assert_eq!(fetched.1.nwk_s_key, session.nwk_s_key);
    }
}
