//! Cloud-side ports for future hub ingestion. Edge runtime does not depend on this crate.

use async_trait::async_trait;
use maverick_extension_contracts::SyncBatchEnvelopeV1;

/// Hub accepts durable batches from edges with idempotent dedup (implementation in v1.x).
#[async_trait]
pub trait HubSyncIngest: Send + Sync {
    async fn accept_batch(&self, batch: &SyncBatchEnvelopeV1) -> Result<(), String>;
}
