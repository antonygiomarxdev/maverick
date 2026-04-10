//! Versioned contracts for future sync and integrations. Edge runtime v1 does not execute sync.

use serde::{Deserialize, Serialize};

/// Semantic version of the extension contract schema.
pub const EXTENSION_CONTRACT_VERSION: &str = "1.0.0";

/// Batch envelope for edge-to-hub replication (store-and-forward friendly).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyncBatchEnvelopeV1 {
    pub contract_version: String,
    pub edge_id: String,
    pub batch_id: String,
    pub created_at_ms: i64,
    pub events: Vec<SyncEventV1>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyncEventV1 {
    pub correlation_id: String,
    pub entity_type: String,
    pub entity_id: Option<String>,
    pub operation: String,
    pub outcome: String,
    pub metadata: Option<serde_json::Value>,
}
