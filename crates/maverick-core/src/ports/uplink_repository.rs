use async_trait::async_trait;
use maverick_domain::DevAddr;

use crate::error::AppResult;

/// Persisted uplink record (minimal for v1 skeleton).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UplinkRecord {
    pub dev_addr: DevAddr,
    /// Reconstructed 32-bit frame counter (set by IngestUplink use case).
    pub f_cnt: u32,
    /// Wall-clock milliseconds since Unix epoch; required by dedup query.
    pub received_at_ms: i64,
    pub payload: Vec<u8>,
    pub application_id: Option<String>,
    /// Decrypted FRMPayload; None if decryption has not been performed or keys are absent.
    pub payload_decrypted: Option<Vec<u8>>,
}

#[async_trait]
pub trait UplinkRepository: Send + Sync {
    async fn append(&self, record: &UplinkRecord) -> AppResult<()>;

    /// Returns true if an uplink with the same (dev_addr, f_cnt) was persisted within
    /// the given time window. Used for multi-gateway duplicate suppression (PROT-06).
    ///
    /// `window_ms` is the look-back window in milliseconds (typically 30_000 for 30 s).
    async fn is_duplicate(&self, dev_addr: DevAddr, f_cnt: u32, window_ms: i64) -> AppResult<bool>;
}
