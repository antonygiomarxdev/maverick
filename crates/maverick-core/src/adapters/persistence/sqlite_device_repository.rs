use std::sync::Arc;

use async_trait::async_trait;
use maverick_domain::{AppKey, Device, DeviceClass, DeviceKeys, DeviceState, Eui64, NwkKey};

use crate::adapters::persistence::sqlite_utils::{
    blob_literal, required_blob, required_i64, required_text, text_literal,
};
use crate::db::{Database, Row};
use crate::error::{AppError, DomainError, Result};
use crate::ports::DeviceRepository;

pub struct SqliteDeviceRepository<D: Database> {
    db: Arc<D>,
}

impl<D: Database> SqliteDeviceRepository<D> {
    pub fn new(db: Arc<D>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl<D: Database> DeviceRepository for SqliteDeviceRepository<D> {
    async fn create(&self, device: Device) -> Result<Device> {
        let query = format!(
            "INSERT INTO devices (dev_eui, app_eui, app_key, nwk_key, device_class, device_state, f_cnt_up, f_cnt_down) VALUES ({}, {}, {}, {}, {}, {}, {}, {})",
            blob_literal(device.dev_eui.as_bytes_slice()),
            blob_literal(device.app_eui.as_bytes_slice()),
            blob_literal(&device.keys.app_key.as_bytes()),
            blob_literal(&device.keys.nwk_key.as_bytes()),
            text_literal(device_class_name(device.class)),
            text_literal(device_state_name(device.state)),
            device.f_cnt_up,
            device.f_cnt_down,
        );
        let dev_eui_str = device.dev_eui.to_string();
        self.db.execute(&query).await.map_err(|e| match e {
            AppError::ConstraintViolation(_) => AppError::Domain(DomainError::AlreadyExists {
                entity: "device",
                id: dev_eui_str,
            }),
            other => other,
        })?;
        Ok(device)
    }

    async fn get_by_dev_eui(&self, dev_eui: Eui64) -> Result<Option<Device>> {
        let query = format!(
            "SELECT dev_eui, app_eui, app_key, nwk_key, device_class, device_state, f_cnt_up, f_cnt_down FROM devices WHERE dev_eui = {} LIMIT 1",
            blob_literal(dev_eui.as_bytes_slice())
        );
        let rows = self.db.query(&query).await?;

        rows.into_iter().next().map(device_from_row).transpose()
    }

    async fn update(&self, device: Device) -> Result<Device> {
        let query = format!(
            "UPDATE devices SET app_eui = {}, app_key = {}, nwk_key = {}, device_class = {}, device_state = {}, f_cnt_up = {}, f_cnt_down = {}, updated_at = unixepoch() WHERE dev_eui = {}",
            blob_literal(device.app_eui.as_bytes_slice()),
            blob_literal(&device.keys.app_key.as_bytes()),
            blob_literal(&device.keys.nwk_key.as_bytes()),
            text_literal(device_class_name(device.class)),
            text_literal(device_state_name(device.state)),
            device.f_cnt_up,
            device.f_cnt_down,
            blob_literal(device.dev_eui.as_bytes_slice()),
        );

        let result = self.db.execute(&query).await?;
        if result.affected_rows == 0 {
            return Err(AppError::Database(
                "device update affected no rows".to_string(),
            ));
        }

        Ok(device)
    }

    async fn delete(&self, dev_eui: Eui64) -> Result<()> {
        let query = format!(
            "DELETE FROM devices WHERE dev_eui = {}",
            blob_literal(dev_eui.as_bytes_slice())
        );
        self.db.execute(&query).await?;
        Ok(())
    }
}

fn device_from_row(row: Row) -> Result<Device> {
    let dev_eui = Eui64::from(required_blob::<8>(&row, 0, "dev_eui")?);
    let app_eui = Eui64::from(required_blob::<8>(&row, 1, "app_eui")?);
    let app_key = AppKey::from(required_blob::<16>(&row, 2, "app_key")?);
    let nwk_key = NwkKey::from(required_blob::<16>(&row, 3, "nwk_key")?);
    let class = parse_device_class(&required_text(&row, 4, "device_class")?)?;
    let state = parse_device_state(&required_text(&row, 5, "device_state")?)?;
    let f_cnt_up = required_i64(&row, 6, "f_cnt_up")? as u32;
    let f_cnt_down = required_i64(&row, 7, "f_cnt_down")? as u32;

    Ok(Device {
        dev_eui,
        app_eui,
        keys: DeviceKeys::new(app_key, nwk_key),
        session: None,
        dev_nonce: None,
        class,
        state,
        f_cnt_up,
        f_cnt_down,
    })
}

fn device_class_name(class: DeviceClass) -> &'static str {
    match class {
        DeviceClass::ClassA => "ClassA",
        DeviceClass::ClassB => "ClassB",
        DeviceClass::ClassC => "ClassC",
    }
}

fn device_state_name(state: DeviceState) -> &'static str {
    match state {
        DeviceState::Init => "Init",
        DeviceState::JoinPending => "JoinPending",
        DeviceState::Active => "Active",
        DeviceState::Sleep => "Sleep",
        DeviceState::Dead => "Dead",
    }
}

fn parse_device_class(value: &str) -> Result<DeviceClass> {
    match value {
        "ClassA" => Ok(DeviceClass::ClassA),
        "ClassB" => Ok(DeviceClass::ClassB),
        "ClassC" => Ok(DeviceClass::ClassC),
        _ => Err(AppError::Database(format!(
            "invalid device class '{value}'"
        ))),
    }
}

fn parse_device_state(value: &str) -> Result<DeviceState> {
    match value {
        "Init" => Ok(DeviceState::Init),
        "JoinPending" => Ok(DeviceState::JoinPending),
        "Active" => Ok(DeviceState::Active),
        "Sleep" => Ok(DeviceState::Sleep),
        "Dead" => Ok(DeviceState::Dead),
        _ => Err(AppError::Database(format!(
            "invalid device state '{value}'"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use maverick_domain::{AppKey, Device, DeviceClass, DeviceKeys, Eui64, NwkKey};

    use crate::adapters::persistence::SqliteDeviceRepository;
    use crate::db::SqliteDb;
    use crate::ports::DeviceRepository;

    #[tokio::test]
    async fn sqlite_device_repository_round_trip() {
        let db = Arc::new(SqliteDb::in_memory().await.expect("db must open"));
        let repository = SqliteDeviceRepository::new(db);
        let mut device = Device::new(
            Eui64::from([1, 2, 3, 4, 5, 6, 7, 8]),
            Eui64::from([8, 7, 6, 5, 4, 3, 2, 1]),
            DeviceKeys::new(AppKey::from([0xAA; 16]), NwkKey::from([0xBB; 16])),
        );
        device.class = DeviceClass::ClassC;

        repository
            .create(device.clone())
            .await
            .expect("create must succeed");

        let fetched = repository
            .get_by_dev_eui(device.dev_eui)
            .await
            .expect("query must succeed")
            .expect("device must exist");

        assert_eq!(fetched.dev_eui.as_bytes(), device.dev_eui.as_bytes());
        assert_eq!(fetched.app_eui.as_bytes(), device.app_eui.as_bytes());
        assert_eq!(
            fetched.keys.app_key.as_bytes(),
            device.keys.app_key.as_bytes()
        );
        assert_eq!(
            fetched.keys.nwk_key.as_bytes(),
            device.keys.nwk_key.as_bytes()
        );
        assert_eq!(fetched.class, DeviceClass::ClassC);
    }
}
