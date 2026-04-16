//! Stable defaults and health component identifiers for the edge binary.

pub const DEFAULT_DATA_DIR: &str = "data";
pub const EDGE_DB_FILENAME: &str = "maverick.db";

pub const HEALTH_COMPONENT_STORAGE: &str = "storage";

/// Default target for `radio downlink-probe` (loopback).
pub const DEFAULT_RADIO_PROBE_HOST: &str = "127.0.0.1";

/// Default UDP port for `radio downlink-probe` (non-privileged; override for real gateways).
pub const DEFAULT_RADIO_PROBE_PORT: u16 = 17_000;

/// Placeholder until log tail is wired to structured files.
pub const RECENT_ERRORS_NOT_WIRED_MESSAGE: &str = "recent-errors not yet wired to log file";

/// Prefix for storage health detail when SQLite open fails.
pub const STORAGE_OPEN_FAILED_PREFIX: &str = "open failed: ";

/// Single-byte UDP probe payload (Semtech GWMP parsing is future work).
pub const RADIO_PROBE_PAYLOAD_BYTE: u8 = 0x01;

/// Default bind address for GWMP uplink ingest.
///
/// Changed from 0.0.0.0 to 127.0.0.1 (SEC-01) — binds to loopback only.
/// For external packet forwarders on other hosts, override with:
///   --bind 0.0.0.0:17000
/// or set MAVERICK_GWMP_BIND=0.0.0.0:17000 in the environment.
pub const DEFAULT_GWMP_BIND_ADDR: &str = "127.0.0.1:17000";

/// Timeout window for one-shot GWMP listen mode in CLI.
pub const DEFAULT_GWMP_INGEST_TIMEOUT_MS: u64 = 5_000;

/// Default socket read timeout for supervised ingest loop.
pub const DEFAULT_GWMP_LOOP_READ_TIMEOUT_MS: u64 = 1_000;

/// Default cap to process in one supervised run; prevents unbounded CLI sessions.
pub const DEFAULT_GWMP_LOOP_MAX_MESSAGES: u32 = 1_000;
