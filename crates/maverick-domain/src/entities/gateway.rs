use crate::types::Eui64;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GatewayStatus {
    Online,
    Offline,
    Timeout,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoLocation {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: Option<f64>,
}

impl GeoLocation {
    pub fn new(latitude: f64, longitude: f64, altitude: Option<f64>) -> Self {
        Self {
            latitude,
            longitude,
            altitude,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gateway {
    pub gateway_eui: Eui64,
    pub location: Option<GeoLocation>,
    pub status: GatewayStatus,
    pub tx_frequency: Option<u32>,
    pub rx_temperature: Option<f32>,
    pub tx_temperature: Option<f32>,
    pub platform: Option<String>,
    pub bridge_ip: Option<String>,
    pub last_seen: Option<i64>,
}

impl Gateway {
    pub fn new(gateway_eui: Eui64) -> Self {
        Self {
            gateway_eui,
            location: None,
            status: GatewayStatus::Offline,
            tx_frequency: None,
            rx_temperature: None,
            tx_temperature: None,
            platform: None,
            bridge_ip: None,
            last_seen: None,
        }
    }

    pub fn with_location(mut self, location: GeoLocation) -> Self {
        self.location = Some(location);
        self
    }

    pub fn is_online(&self) -> bool {
        self.status == GatewayStatus::Online
    }

    pub fn update_status(&mut self, status: GatewayStatus) {
        self.status = status;
    }
}
