//! Operational limits for UDP transport and resilience (no scattered numeric literals).

use std::time::Duration;

/// Default per-attempt I/O timeout for downlink send.
pub(crate) const DEFAULT_PER_ATTEMPT_TIMEOUT: Duration = Duration::from_secs(2);

/// Default maximum retries after the first attempt (total attempts = 1 + this value).
pub(crate) const DEFAULT_MAX_RETRIES: u32 = 2;

/// Base backoff between retry attempts.
pub(crate) const DEFAULT_BACKOFF_BASE: Duration = Duration::from_millis(50);

/// Upper cap for exponential backoff.
pub(crate) const DEFAULT_BACKOFF_MAX: Duration = Duration::from_millis(500);

/// Consecutive failed operations (after retries) before the circuit trips open.
pub(crate) const DEFAULT_CIRCUIT_FAILURE_THRESHOLD: u32 = 3;

/// How long the circuit stays open before allowing a trial again.
pub(crate) const DEFAULT_CIRCUIT_OPEN_DURATION: Duration = Duration::from_secs(5);
