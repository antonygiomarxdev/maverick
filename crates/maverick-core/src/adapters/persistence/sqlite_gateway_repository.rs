use std::sync::Arc;

use async_trait::async_trait;
use maverick_domain::{Eui64, Gateway, GatewayStatus, GeoLocation};

use crate::adapters::persistence::sqlite_utils::{
    blob_literal, optional_i64, optional_real, optional_text, optional_text_literal, required_blob,
    required_text,
};
use crate::db::{Database, Row};
use crate::error::{AppError, DomainError, Result};
use crate::ports::GatewayRepository;

#[derive(Clone)]
pub struct SqliteGatewayRepository<D: Database> {
    db: Arc<D>,
}

impl<D: Database> SqliteGatewayRepository<D> {
    pub fn new(db: Arc<D>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl<D: Database> GatewayRepository for SqliteGatewayRepository<D> {
    async fn create(&self, gateway: Gateway) -> Result<Gateway> {
        let eui_str = gateway.gateway_eui.to_string();
        let (latitude, longitude, altitude) = location_parts(&gateway);
        let query = format!(
            "INSERT INTO gateways (gateway_eui, status, latitude, longitude, altitude, tx_frequency, rx_temperature, tx_temperature, platform, bridge_ip, last_seen, created_at, updated_at) VALUES ({}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, unixepoch(), unixepoch())",
            blob_literal(gateway.gateway_eui.as_bytes_slice()),
            optional_text_literal(Some(gateway_status_name(gateway.status))),
            latitude,
            longitude,
            altitude,
            optional_i64(gateway.tx_frequency.map(|v| v as i64)),
            optional_real(gateway.rx_temperature.map(|v| v as f64)),
            optional_real(gateway.tx_temperature.map(|v| v as f64)),
            optional_text_literal(gateway.platform.as_deref()),
            optional_text_literal(gateway.bridge_ip.as_deref()),
            optional_i64(gateway.last_seen),
        );
        self.db.execute(&query).await.map_err(|e| match e {
            AppError::ConstraintViolation(_) => AppError::Domain(DomainError::AlreadyExists {
                entity: "gateway",
                id: eui_str,
            }),
            other => other,
        })?;
        Ok(gateway)
    }

    async fn update(&self, gateway: Gateway) -> Result<Gateway> {
        let eui_str = gateway.gateway_eui.to_string();
        let (latitude, longitude, altitude) = location_parts(&gateway);
        let query = format!(
            "UPDATE gateways SET status = {}, latitude = {}, longitude = {}, altitude = {}, tx_frequency = {}, rx_temperature = {}, tx_temperature = {}, platform = {}, bridge_ip = {}, last_seen = {}, updated_at = unixepoch() WHERE gateway_eui = {}",
            optional_text_literal(Some(gateway_status_name(gateway.status))),
            latitude,
            longitude,
            altitude,
            optional_i64(gateway.tx_frequency.map(|v| v as i64)),
            optional_real(gateway.rx_temperature.map(|v| v as f64)),
            optional_real(gateway.tx_temperature.map(|v| v as f64)),
            optional_text_literal(gateway.platform.as_deref()),
            optional_text_literal(gateway.bridge_ip.as_deref()),
            optional_i64(gateway.last_seen),
            blob_literal(gateway.gateway_eui.as_bytes_slice()),
        );
        let result = self.db.execute(&query).await?;
        if result.affected_rows == 0 {
            return Err(AppError::Domain(DomainError::NotFound {
                entity: "gateway",
                id: eui_str,
            }));
        }
        Ok(gateway)
    }

    async fn delete(&self, gateway_eui: Eui64) -> Result<()> {
        let query = format!(
            "DELETE FROM gateways WHERE gateway_eui = {}",
            blob_literal(gateway_eui.as_bytes_slice())
        );
        let result = self.db.execute(&query).await?;
        if result.affected_rows == 0 {
            return Err(AppError::Domain(DomainError::NotFound {
                entity: "gateway",
                id: gateway_eui.to_string(),
            }));
        }
        Ok(())
    }

    async fn get_by_gateway_eui(&self, gateway_eui: Eui64) -> Result<Option<Gateway>> {
        let query = format!(
            "SELECT gateway_eui, status, latitude, longitude, altitude, tx_frequency, rx_temperature, tx_temperature, platform, bridge_ip, last_seen FROM gateways WHERE gateway_eui = {} LIMIT 1",
            blob_literal(gateway_eui.as_bytes_slice())
        );
        let rows = self.db.query(&query).await?;
        rows.into_iter().next().map(gateway_from_row).transpose()
    }

    async fn list(&self, status: Option<GatewayStatus>) -> Result<Vec<Gateway>> {
        let filter = status
            .map(|value| {
                format!(
                    " WHERE status = {}",
                    optional_text_literal(Some(gateway_status_name(value)))
                )
            })
            .unwrap_or_default();
        let query = format!(
            "SELECT gateway_eui, status, latitude, longitude, altitude, tx_frequency, rx_temperature, tx_temperature, platform, bridge_ip, last_seen FROM gateways{filter} ORDER BY CASE status WHEN 'Online' THEN 0 WHEN 'Timeout' THEN 1 ELSE 2 END, COALESCE(last_seen, 0) DESC"
        );
        let rows = self.db.query(&query).await?;
        rows.into_iter().map(gateway_from_row).collect()
    }

    async fn list_healthy(&self) -> Result<Vec<Gateway>> {
        self.list(Some(GatewayStatus::Online)).await
    }
}

fn location_parts(gateway: &Gateway) -> (String, String, String) {
    match &gateway.location {
        Some(loc) => (
            optional_real(Some(loc.latitude)),
            optional_real(Some(loc.longitude)),
            optional_real(loc.altitude),
        ),
        None => ("NULL".to_string(), "NULL".to_string(), "NULL".to_string()),
    }
}

fn gateway_from_row(row: Row) -> Result<Gateway> {
    let gateway_eui = Eui64::from(required_blob::<8>(&row, 0, "gateway_eui")?);
    let status = parse_gateway_status(&required_text(&row, 1, "status")?)?;
    let latitude = optional_real_value(&row, 2);
    let longitude = optional_real_value(&row, 3);
    let altitude = optional_real_value(&row, 4);
    let location = match (latitude, longitude) {
        (Some(latitude), Some(longitude)) => Some(GeoLocation::new(latitude, longitude, altitude)),
        _ => None,
    };

    Ok(Gateway {
        gateway_eui,
        location,
        status,
        tx_frequency: optional_i64_value(&row, 5).map(|value| value as u32),
        rx_temperature: optional_real_value(&row, 6).map(|value| value as f32),
        tx_temperature: optional_real_value(&row, 7).map(|value| value as f32),
        platform: optional_text(&row, 8),
        bridge_ip: optional_text(&row, 9),
        last_seen: optional_i64_value(&row, 10),
    })
}

fn gateway_status_name(status: GatewayStatus) -> &'static str {
    match status {
        GatewayStatus::Online => "Online",
        GatewayStatus::Offline => "Offline",
        GatewayStatus::Timeout => "Timeout",
    }
}

fn parse_gateway_status(value: &str) -> Result<GatewayStatus> {
    match value {
        "Online" => Ok(GatewayStatus::Online),
        "Offline" => Ok(GatewayStatus::Offline),
        "Timeout" => Ok(GatewayStatus::Timeout),
        _ => Err(AppError::Database(format!(
            "invalid gateway status '{value}'"
        ))),
    }
}

fn optional_real_value(row: &Row, index: usize) -> Option<f64> {
    match row.values.get(index) {
        Some(crate::db::Value::Real(value)) => Some(*value),
        Some(crate::db::Value::Integer(value)) => Some(*value as f64),
        _ => None,
    }
}

fn optional_i64_value(row: &Row, index: usize) -> Option<i64> {
    match row.values.get(index) {
        Some(crate::db::Value::Integer(value)) => Some(*value),
        _ => None,
    }
}
