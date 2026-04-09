use std::collections::VecDeque;
use std::sync::Arc;

use async_trait::async_trait;
use maverick_domain::UplinkFrame;
use tokio::sync::Mutex;

use crate::events::{EventBus, EventKind, EventSource, EventStatus, SystemEvent};
use crate::ports::UplinkRepository;
use crate::Result;

/// In-memory circular uplink buffer for the Extreme storage profile.
/// Drops the oldest frame when capacity is reached, preserving continuity
/// at the cost of historical completeness.
pub struct CircularUplinkBuffer {
    inner: Arc<Mutex<VecDeque<UplinkFrame>>>,
    capacity: usize,
    event_bus: EventBus,
}

impl CircularUplinkBuffer {
    pub fn new(capacity: usize, event_bus: EventBus) -> Self {
        Self {
            inner: Arc::new(Mutex::new(VecDeque::with_capacity(capacity))),
            capacity,
            event_bus,
        }
    }

    /// Drain all buffered frames for controlled offload to persistent storage.
    pub async fn drain_pending(&self) -> Vec<UplinkFrame> {
        let mut guard = self.inner.lock().await;
        guard.drain(..).collect()
    }

    /// Current number of buffered frames.
    pub async fn len(&self) -> usize {
        self.inner.lock().await.len()
    }
}

#[async_trait]
impl UplinkRepository for CircularUplinkBuffer {
    async fn append(&self, uplink: UplinkFrame) -> Result<()> {
        let mut guard = self.inner.lock().await;
        let was_full = guard.len() >= self.capacity;
        if was_full {
            guard.pop_front();
        }
        guard.push_back(uplink);
        drop(guard);
        if was_full {
            self.event_bus.publish(
                SystemEvent::new(
                    EventKind::CircularDrop,
                    EventSource::Udp,
                    "storage.circular.frame_dropped",
                    EventStatus::Rejected,
                )
                .with_reason_code("circular_buffer_full"),
            );
        }
        Ok(())
    }

    async fn append_batch(&self, uplinks: Vec<UplinkFrame>) -> Result<()> {
        for uplink in uplinks {
            self.append(uplink).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maverick_domain::{Eui64, Frequency, Rssi, Snr, SpreadingFactor};

    fn make_frame(seed: u8) -> UplinkFrame {
        UplinkFrame::new(
            Eui64::from([seed; 8]),
            vec![seed],
            Rssi::new(-80),
            Snr::new(7.0),
            Frequency::new(868_100_000),
            SpreadingFactor::new(7).unwrap(),
            0,
            vec![seed],
        )
    }

    fn make_bus() -> EventBus {
        EventBus::new(16)
    }

    #[tokio::test]
    async fn append_respects_capacity() {
        let buf = CircularUplinkBuffer::new(3, make_bus());
        for i in 0..3u8 {
            buf.append(make_frame(i)).await.unwrap();
        }
        assert_eq!(buf.len().await, 3);
        buf.append(make_frame(42)).await.unwrap();
        assert_eq!(buf.len().await, 3);
        let drained = buf.drain_pending().await;
        assert_eq!(drained.len(), 3);
        assert_eq!(drained[0].payload, vec![1u8]);
        assert_eq!(drained[2].payload, vec![42u8]);
    }

    #[tokio::test]
    async fn drain_clears_buffer() {
        let buf = CircularUplinkBuffer::new(5, make_bus());
        buf.append(make_frame(1)).await.unwrap();
        buf.append(make_frame(2)).await.unwrap();
        assert_eq!(buf.len().await, 2);
        let drained = buf.drain_pending().await;
        assert_eq!(drained.len(), 2);
        assert_eq!(buf.len().await, 0);
    }

    #[tokio::test]
    async fn append_batch_respects_capacity() {
        let buf = CircularUplinkBuffer::new(2, make_bus());
        let frames: Vec<UplinkFrame> = (0..5u8).map(make_frame).collect();
        buf.append_batch(frames).await.unwrap();
        assert_eq!(buf.len().await, 2);
        let drained = buf.drain_pending().await;
        assert_eq!(drained[0].payload, vec![3u8]);
        assert_eq!(drained[1].payload, vec![4u8]);
    }
}
