//! Timeout, bounded retry with backoff, and a simple circuit breaker around any [`RadioTransport`].

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use maverick_core::error::{AppError, AppResult};
use maverick_core::ports::{DownlinkFrame, RadioTransport};
use tokio::time::sleep;

use crate::limits::{
    DEFAULT_BACKOFF_BASE, DEFAULT_BACKOFF_MAX, DEFAULT_CIRCUIT_FAILURE_THRESHOLD,
    DEFAULT_CIRCUIT_OPEN_DURATION, DEFAULT_MAX_RETRIES, DEFAULT_PER_ATTEMPT_TIMEOUT,
};

/// Tunable resilience policy for outbound radio I/O.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResiliencePolicy {
    pub per_attempt_timeout: Duration,
    pub max_retries: u32,
    pub backoff_base: Duration,
    pub backoff_max: Duration,
    pub circuit_failure_threshold: u32,
    pub circuit_open_duration: Duration,
}

impl Default for ResiliencePolicy {
    fn default() -> Self {
        Self {
            per_attempt_timeout: DEFAULT_PER_ATTEMPT_TIMEOUT,
            max_retries: DEFAULT_MAX_RETRIES,
            backoff_base: DEFAULT_BACKOFF_BASE,
            backoff_max: DEFAULT_BACKOFF_MAX,
            circuit_failure_threshold: DEFAULT_CIRCUIT_FAILURE_THRESHOLD,
            circuit_open_duration: DEFAULT_CIRCUIT_OPEN_DURATION,
        }
    }
}

struct CircuitState {
    open_until: Option<Instant>,
}

/// Wraps an inner [`RadioTransport`] with timeout/retry/backoff and a circuit breaker.
pub struct ResilientRadioTransport {
    inner: Arc<dyn RadioTransport>,
    policy: ResiliencePolicy,
    circuit: Mutex<CircuitState>,
    consecutive_failures: AtomicU32,
}

impl ResilientRadioTransport {
    pub fn new(inner: Arc<dyn RadioTransport>, policy: ResiliencePolicy) -> Self {
        Self {
            inner,
            policy,
            circuit: Mutex::new(CircuitState { open_until: None }),
            consecutive_failures: AtomicU32::new(0),
        }
    }

    fn circuit_blocks_now(&self) -> Option<Duration> {
        let mut g = self.circuit.lock().ok()?;
        let now = Instant::now();
        if let Some(until) = g.open_until {
            if now < until {
                return Some(until - now);
            }
            g.open_until = None;
        }
        None
    }

    fn record_success(&self) {
        self.consecutive_failures.store(0, Ordering::SeqCst);
        if let Ok(mut g) = self.circuit.lock() {
            g.open_until = None;
        }
    }

    fn record_failure_after_retries(&self) {
        let n = self
            .consecutive_failures
            .fetch_add(1, Ordering::SeqCst)
            .saturating_add(1);
        if n >= self.policy.circuit_failure_threshold {
            if let Ok(mut g) = self.circuit.lock() {
                g.open_until = Some(Instant::now() + self.policy.circuit_open_duration);
            }
            self.consecutive_failures.store(0, Ordering::SeqCst);
        }
    }

    fn backoff_delay(policy: ResiliencePolicy, retry_index: u32) -> Duration {
        let shift = retry_index.min(16);
        policy
            .backoff_base
            .checked_mul(1u32.checked_shl(shift).unwrap_or(u32::MAX))
            .unwrap_or(policy.backoff_max)
            .min(policy.backoff_max)
    }
}

#[async_trait]
impl RadioTransport for ResilientRadioTransport {
    async fn send_downlink(&self, frame: &DownlinkFrame) -> AppResult<()> {
        if let Some(remaining) = self.circuit_blocks_now() {
            return Err(AppError::CircuitOpen(format!(
                "radio transport circuit open; retry after {} ms",
                remaining.as_millis()
            )));
        }

        let mut last_err: Option<AppError> = None;
        let total_attempts = self.policy.max_retries.saturating_add(1).max(1);

        for attempt in 0..total_attempts {
            if attempt > 0 {
                sleep(Self::backoff_delay(self.policy, attempt - 1)).await;
            }

            let inner = self.inner.clone();
            let frame = frame.clone();
            let timeout_d = self.policy.per_attempt_timeout;

            match tokio::time::timeout(timeout_d, async move { inner.send_downlink(&frame).await })
                .await
            {
                Ok(Ok(())) => {
                    self.record_success();
                    return Ok(());
                }
                Ok(Err(e)) => {
                    last_err = Some(e);
                }
                Err(_elapsed) => {
                    last_err = Some(AppError::Infrastructure(format!(
                        "radio transport timeout after {} ms",
                        timeout_d.as_millis()
                    )));
                }
            }
        }

        self.record_failure_after_retries();
        Err(last_err.unwrap_or_else(|| {
            AppError::Infrastructure("radio transport failed with no error captured".to_string())
        }))
    }
}

#[cfg(test)]
mod tests {
    use std::future::pending;
    use std::sync::Arc;
    use std::time::Duration;

    use async_trait::async_trait;
    use maverick_core::error::{AppError, AppResult};
    use maverick_core::ports::{DownlinkFrame, RadioTransport};
    use maverick_domain::identifiers::Eui64;
    use maverick_domain::{DevAddr, GatewayEui};

    use super::{ResiliencePolicy, ResilientRadioTransport};

    struct Hang;
    #[async_trait]
    impl RadioTransport for Hang {
        async fn send_downlink(&self, _frame: &DownlinkFrame) -> AppResult<()> {
            pending::<()>().await;
            Ok(())
        }
    }

    struct AlwaysFail;
    #[async_trait]
    impl RadioTransport for AlwaysFail {
        async fn send_downlink(&self, _frame: &DownlinkFrame) -> AppResult<()> {
            Err(AppError::Infrastructure(
                "injected transport failure".to_string(),
            ))
        }
    }

    fn sample_frame() -> DownlinkFrame {
        DownlinkFrame {
            gateway_eui: GatewayEui(Eui64([0_u8; 8])),
            dev_addr: DevAddr(1),
            payload: vec![0xAB],
        }
    }

    #[tokio::test]
    async fn times_out_when_inner_never_completes() {
        let inner: Arc<dyn RadioTransport> = Arc::new(Hang);
        let policy = ResiliencePolicy {
            per_attempt_timeout: Duration::from_millis(40),
            max_retries: 0,
            ..ResiliencePolicy::default()
        };
        let transport = ResilientRadioTransport::new(inner, policy);
        let err = transport
            .send_downlink(&sample_frame())
            .await
            .expect_err("expected timeout");
        match err {
            AppError::Infrastructure(msg) => {
                assert!(msg.contains("timeout"), "unexpected infra message: {msg}")
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[tokio::test]
    async fn circuit_opens_after_repeated_failures() {
        let inner: Arc<dyn RadioTransport> = Arc::new(AlwaysFail);
        let policy = ResiliencePolicy {
            per_attempt_timeout: Duration::from_millis(20),
            max_retries: 0,
            circuit_failure_threshold: 2,
            circuit_open_duration: Duration::from_secs(120),
            ..ResiliencePolicy::default()
        };
        let transport = ResilientRadioTransport::new(inner, policy);
        let frame = sample_frame();

        let _ = transport.send_downlink(&frame).await;
        let _ = transport.send_downlink(&frame).await;
        let err = transport
            .send_downlink(&frame)
            .await
            .expect_err("expected circuit open");
        assert!(matches!(err, AppError::CircuitOpen(_)), "got {err:?}");
    }
}
