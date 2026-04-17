PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;
PRAGMA synchronous = NORMAL;

CREATE TABLE IF NOT EXISTS sessions (
    dev_addr INTEGER PRIMARY KEY NOT NULL,
    dev_eui BLOB NOT NULL,
    region TEXT NOT NULL,
    device_class TEXT NOT NULL,
    uplink_fcnt INTEGER NOT NULL,
    downlink_fcnt INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    application_id TEXT,
    -- AES-128 session keys; stored as plaintext BLOB (SEC-02: application-level encryption deferred - requires domain model changes to support encrypted key storage)
    nwk_s_key BLOB NOT NULL,
    app_s_key BLOB NOT NULL
);

CREATE TABLE IF NOT EXISTS uplinks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    dev_addr INTEGER NOT NULL,
    f_cnt INTEGER NOT NULL,
    received_at_ms INTEGER NOT NULL,
    payload BLOB NOT NULL,
    payload_decrypted BLOB,
    application_id TEXT
);
CREATE INDEX IF NOT EXISTS idx_uplinks_id ON uplinks(id);
CREATE INDEX IF NOT EXISTS idx_uplinks_dedup ON uplinks(dev_addr, f_cnt, received_at_ms);

CREATE TABLE IF NOT EXISTS audit_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source TEXT NOT NULL,
    operation TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id TEXT,
    outcome TEXT NOT NULL,
    metadata TEXT,
    created_at_ms INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_audit_id ON audit_events(id);

-- Declarative LNS config mirror (synced from `/etc/maverick/lns-config.toml` via `maverick-edge config load`).
CREATE TABLE IF NOT EXISTS lns_applications (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL DEFAULT '',
    default_region TEXT NOT NULL
);

-- LNS device mirror: `dev_addr` may be NULL for OTAA until a session is known.
CREATE TABLE IF NOT EXISTS lns_devices (
    dev_eui BLOB NOT NULL PRIMARY KEY,
    dev_addr INTEGER UNIQUE,
    activation_mode TEXT NOT NULL,
    application_id TEXT NOT NULL,
    region TEXT NOT NULL,
    enabled INTEGER NOT NULL,
    join_eui BLOB,
    app_key BLOB,
    nwk_key BLOB,
    apps_key BLOB,
    nwks_key BLOB
);
CREATE INDEX IF NOT EXISTS idx_lns_devices_dev_addr ON lns_devices(dev_addr);

CREATE TABLE IF NOT EXISTS lns_pending (
    dev_addr INTEGER PRIMARY KEY NOT NULL,
    gateway_eui BLOB NOT NULL,
    first_seen_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS lns_meta (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    autoprovision_enabled INTEGER NOT NULL,
    rate_limit_per_gateway_per_minute INTEGER NOT NULL,
    pending_ttl_secs INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS downlink_queue (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    dev_eui BLOB NOT NULL,
    dev_addr INTEGER NOT NULL,
    f_port INTEGER NOT NULL,
    payload BLOB NOT NULL,
    confirmed INTEGER NOT NULL DEFAULT 0,
    ack_flag INTEGER NOT NULL DEFAULT 0,
    enqueued_at_ms INTEGER NOT NULL,
    frame_counter INTEGER NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    transmitted_at_ms INTEGER
);
CREATE INDEX IF NOT EXISTS idx_downlink_dev_eui ON downlink_queue(dev_eui);
CREATE INDEX IF NOT EXISTS idx_downlink_status ON downlink_queue(status);
