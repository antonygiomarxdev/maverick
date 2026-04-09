use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use maverick_domain::UplinkFrame;
use tokio::sync::Mutex;

use crate::error::Result;
use crate::ports::UplinkRepository;

/// Accumulates uplink frames and flushes them in a single transaction,
/// reducing write amplification on storage-constrained devices.
pub struct BatchWriter<U: UplinkRepository> {
    repo: Arc<U>,
    pending: Arc<Mutex<Vec<UplinkFrame>>>,
    threshold: usize,
}

impl<U: UplinkRepository> Clone for BatchWriter<U> {
    fn clone(&self) -> Self {
        Self {
            repo: Arc::clone(&self.repo),
            pending: Arc::clone(&self.pending),
            threshold: self.threshold,
        }
    }
}

impl<U: UplinkRepository + 'static> BatchWriter<U> {
    pub fn new(repo: Arc<U>, threshold: usize) -> Self {
        Self {
            repo,
            pending: Arc::new(Mutex::new(Vec::new())),
            threshold,
        }
    }

    /// Append an uplink frame; auto-flushes when the threshold is reached.
    pub async fn push(&self, uplink: UplinkFrame) {
        let mut guard = self.pending.lock().await;
        guard.push(uplink);
        if guard.len() >= self.threshold {
            let batch: Vec<UplinkFrame> = guard.drain(..).collect();
            drop(guard);
            self.write_batch(batch).await;
        }
    }

    /// Flush all pending frames immediately.
    pub async fn flush(&self) {
        let mut guard = self.pending.lock().await;
        if guard.is_empty() {
            return;
        }
        let batch: Vec<UplinkFrame> = guard.drain(..).collect();
        drop(guard);
        self.write_batch(batch).await;
    }

    /// Spawn a background task that flushes on a fixed interval.
    pub fn spawn_drain_loop(self, interval: Duration) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            loop {
                ticker.tick().await;
                self.flush().await;
            }
        })
    }

    async fn write_batch(&self, batch: Vec<UplinkFrame>) {
        if let Err(e) = self.repo.append_batch(batch).await {
            tracing::warn!(error = %e, "batch writer flush error");
        }
    }
}

#[async_trait]
impl<U: UplinkRepository + 'static> UplinkRepository for BatchWriter<U> {
    async fn append(&self, uplink: UplinkFrame) -> Result<()> {
        self.push(uplink).await;
        Ok(())
    }

    async fn append_batch(&self, uplinks: Vec<UplinkFrame>) -> Result<()> {
        for uplink in uplinks {
            self.push(uplink).await;
        }
        Ok(())
    }
}
