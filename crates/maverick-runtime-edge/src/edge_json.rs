//! Stable JSON field names and small payloads for `maverick-edge` CLI output.

use serde::Serialize;
use serde_json::{json, Map, Value};

/// JSON object keys emitted by the edge CLI (`serde_json` keys are centralized here).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EdgeJsonKey {
    Role,
    DataDir,
    SuggestedProfile,
    MemoryBytes,
    Storage,
    Present,
    Level,
    DbBytes,
    TotalDiskBytes,
    Detail,
    Error,
    Outcome,
    GatewayHost,
    GatewayPort,
    PayloadBytes,
    ListenBind,
    TimeoutMs,
    Received,
    Parsed,
    Ingested,
    Failed,
    Looped,
}

impl EdgeJsonKey {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            EdgeJsonKey::Role => "role",
            EdgeJsonKey::DataDir => "data_dir",
            EdgeJsonKey::SuggestedProfile => "suggested_profile",
            EdgeJsonKey::MemoryBytes => "memory_bytes",
            EdgeJsonKey::Storage => "storage",
            EdgeJsonKey::Present => "present",
            EdgeJsonKey::Level => "level",
            EdgeJsonKey::DbBytes => "db_bytes",
            EdgeJsonKey::TotalDiskBytes => "total_disk_bytes",
            EdgeJsonKey::Detail => "detail",
            EdgeJsonKey::Error => "error",
            EdgeJsonKey::Outcome => "outcome",
            EdgeJsonKey::GatewayHost => "gateway_host",
            EdgeJsonKey::GatewayPort => "gateway_port",
            EdgeJsonKey::PayloadBytes => "payload_bytes",
            EdgeJsonKey::ListenBind => "listen_bind",
            EdgeJsonKey::TimeoutMs => "timeout_ms",
            EdgeJsonKey::Received => "received",
            EdgeJsonKey::Parsed => "parsed",
            EdgeJsonKey::Ingested => "ingested",
            EdgeJsonKey::Failed => "failed",
            EdgeJsonKey::Looped => "looped",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EdgeProcessRole {
    Edge,
}

impl EdgeProcessRole {
    const fn as_str(self) -> &'static str {
        match self {
            EdgeProcessRole::Edge => "edge",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RadioProbeOutcome {
    Sent,
    Failed,
}

impl RadioProbeOutcome {
    const fn as_str(self) -> &'static str {
        match self {
            RadioProbeOutcome::Sent => "sent",
            RadioProbeOutcome::Failed => "failed",
        }
    }
}

fn key(k: EdgeJsonKey) -> String {
    k.as_str().to_string()
}

pub(crate) fn status_document(
    data_dir: &std::path::Path,
    suggested_profile: String,
    memory_bytes: u64,
    storage: Value,
) -> Value {
    let mut root = Map::new();
    root.insert(
        key(EdgeJsonKey::Role),
        json!(EdgeProcessRole::Edge.as_str()),
    );
    root.insert(
        key(EdgeJsonKey::DataDir),
        json!(data_dir.display().to_string()),
    );
    root.insert(key(EdgeJsonKey::SuggestedProfile), json!(suggested_profile));
    root.insert(key(EdgeJsonKey::MemoryBytes), json!(memory_bytes));
    root.insert(key(EdgeJsonKey::Storage), storage);
    Value::Object(root)
}

pub(crate) fn storage_absent() -> Value {
    let mut m = Map::new();
    m.insert(key(EdgeJsonKey::Present), json!(false));
    Value::Object(m)
}

pub(crate) fn storage_pressure_absent(data_dir: &std::path::Path) -> Value {
    let mut m = Map::new();
    m.insert(key(EdgeJsonKey::Present), json!(false));
    m.insert(
        key(EdgeJsonKey::DataDir),
        json!(data_dir.display().to_string()),
    );
    Value::Object(m)
}

pub(crate) fn storage_present_ok(snap: &maverick_core::StoragePressureSnapshot) -> Value {
    let mut m = Map::new();
    m.insert(key(EdgeJsonKey::Present), json!(true));
    m.insert(key(EdgeJsonKey::Level), json!(snap.level));
    m.insert(key(EdgeJsonKey::DbBytes), json!(snap.db_bytes));
    m.insert(
        key(EdgeJsonKey::TotalDiskBytes),
        json!(snap.total_disk_bytes),
    );
    m.insert(key(EdgeJsonKey::Detail), json!(snap.detail));
    Value::Object(m)
}

pub(crate) fn storage_present_err(err: &maverick_core::AppError) -> Value {
    let mut m = Map::new();
    m.insert(key(EdgeJsonKey::Present), json!(true));
    m.insert(key(EdgeJsonKey::Error), json!(err.to_string()));
    Value::Object(m)
}

pub(crate) fn storage_pressure_open_err(err: &maverick_core::AppError) -> Value {
    let mut m = Map::new();
    m.insert(key(EdgeJsonKey::Error), json!(err.to_string()));
    Value::Object(m)
}

#[derive(Serialize)]
pub(crate) struct RecentErrorsStubResponse {
    pub message: &'static str,
    pub lines_requested: usize,
}

pub(crate) fn radio_probe_result(
    outcome: RadioProbeOutcome,
    host: &str,
    port: u16,
    payload_len: usize,
    detail: Option<String>,
) -> Value {
    let mut m = Map::new();
    m.insert(key(EdgeJsonKey::Outcome), json!(outcome.as_str()));
    m.insert(key(EdgeJsonKey::GatewayHost), json!(host));
    m.insert(key(EdgeJsonKey::GatewayPort), json!(port));
    m.insert(key(EdgeJsonKey::PayloadBytes), json!(payload_len));
    if let Some(d) = detail {
        m.insert(key(EdgeJsonKey::Detail), json!(d));
    }
    Value::Object(m)
}

pub(crate) fn radio_ingest_result(
    bind: &str,
    timeout_ms: u64,
    received: usize,
    parsed: usize,
    ingested: usize,
    failed: usize,
    detail: Option<String>,
) -> Value {
    let mut m = Map::new();
    m.insert(key(EdgeJsonKey::ListenBind), json!(bind));
    m.insert(key(EdgeJsonKey::TimeoutMs), json!(timeout_ms));
    m.insert(key(EdgeJsonKey::Received), json!(received));
    m.insert(key(EdgeJsonKey::Parsed), json!(parsed));
    m.insert(key(EdgeJsonKey::Ingested), json!(ingested));
    m.insert(key(EdgeJsonKey::Failed), json!(failed));
    if let Some(d) = detail {
        m.insert(key(EdgeJsonKey::Detail), json!(d));
    }
    Value::Object(m)
}

pub(crate) fn radio_ingest_loop_result(
    bind: &str,
    timeout_ms: u64,
    counters: RadioIngestCounters,
    detail: Option<String>,
) -> Value {
    let mut m = Map::new();
    m.insert(key(EdgeJsonKey::ListenBind), json!(bind));
    m.insert(key(EdgeJsonKey::TimeoutMs), json!(timeout_ms));
    m.insert(key(EdgeJsonKey::Looped), json!(counters.looped));
    m.insert(key(EdgeJsonKey::Received), json!(counters.received));
    m.insert(key(EdgeJsonKey::Parsed), json!(counters.parsed));
    m.insert(key(EdgeJsonKey::Ingested), json!(counters.ingested));
    m.insert(key(EdgeJsonKey::Failed), json!(counters.failed));
    if let Some(d) = detail {
        m.insert(key(EdgeJsonKey::Detail), json!(d));
    }
    Value::Object(m)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct RadioIngestCounters {
    pub looped: bool,
    pub received: usize,
    pub parsed: usize,
    pub ingested: usize,
    pub failed: usize,
}
