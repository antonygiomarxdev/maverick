use serde::{Deserialize, Serialize};

/// Overall node health (operator-facing).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

/// Per-component probe result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentHealth {
    pub name: String,
    pub status: HealthStatus,
    pub detail: Option<String>,
}

/// Aggregate health snapshot for CLI / local diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthState {
    pub overall: HealthStatus,
    pub components: Vec<ComponentHealth>,
}

impl HealthState {
    pub fn new(components: Vec<ComponentHealth>) -> Self {
        let overall = if components
            .iter()
            .any(|c| c.status == HealthStatus::Unhealthy)
        {
            HealthStatus::Unhealthy
        } else if components
            .iter()
            .any(|c| c.status == HealthStatus::Degraded)
        {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        };
        Self {
            overall,
            components,
        }
    }
}
