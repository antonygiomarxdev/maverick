CREATE TABLE IF NOT EXISTS devices (
    dev_eui BLOB PRIMARY KEY,
    app_eui BLOB NOT NULL,
    app_key BLOB NOT NULL,
    nwk_key BLOB NOT NULL,
    device_class TEXT NOT NULL DEFAULT 'ClassA',
    device_state TEXT NOT NULL DEFAULT 'Init',
    f_cnt_up INTEGER NOT NULL DEFAULT 0,
    f_cnt_down INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    updated_at INTEGER NOT NULL DEFAULT (unixepoch())
);

CREATE TABLE IF NOT EXISTS gateways (
    gateway_eui BLOB PRIMARY KEY,
    status TEXT NOT NULL DEFAULT 'Offline',
    latitude REAL,
    longitude REAL,
    altitude REAL,
    tx_frequency INTEGER,
    rx_temperature REAL,
    tx_temperature REAL,
    platform TEXT,
    bridge_ip TEXT,
    last_seen INTEGER,
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    updated_at INTEGER NOT NULL DEFAULT (unixepoch())
);

CREATE TABLE IF NOT EXISTS uplinks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    dev_eui BLOB,
    gateway_eui BLOB NOT NULL,
    payload BLOB NOT NULL,
    f_port INTEGER,
    rssi INTEGER NOT NULL,
    snr REAL NOT NULL,
    frequency_hz INTEGER NOT NULL,
    spreading_factor INTEGER NOT NULL,
    frame_counter INTEGER,
    received_at INTEGER NOT NULL,
    raw_frame BLOB,
    channel INTEGER NOT NULL DEFAULT 0,
    code_rate TEXT,
    modulation TEXT,
    bandwidth_hz INTEGER,
    synced INTEGER NOT NULL DEFAULT 0,
    expires_at INTEGER,
    created_at INTEGER NOT NULL DEFAULT (unixepoch())
);

CREATE TABLE IF NOT EXISTS downlinks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    dev_eui BLOB NOT NULL,
    gateway_eui BLOB NOT NULL,
    payload BLOB NOT NULL,
    f_port INTEGER NOT NULL,
    frequency_hz INTEGER NOT NULL,
    spreading_factor INTEGER NOT NULL,
    frame_counter INTEGER NOT NULL,
    priority TEXT NOT NULL DEFAULT 'Normal',
    scheduled_at INTEGER,
    state TEXT NOT NULL DEFAULT 'Queued',
    attempt_count INTEGER NOT NULL DEFAULT 0,
    last_error TEXT,
    sent_at INTEGER,
    updated_at INTEGER NOT NULL DEFAULT (unixepoch()),
    created_at INTEGER NOT NULL DEFAULT (unixepoch())
);

CREATE TABLE IF NOT EXISTS device_sessions (
    dev_eui BLOB PRIMARY KEY,
    dev_addr INTEGER NOT NULL UNIQUE,
    app_s_key BLOB NOT NULL,
    nwk_s_key BLOB NOT NULL,
    frame_counter INTEGER NOT NULL DEFAULT 0,
    last_join_time INTEGER,
    updated_at INTEGER NOT NULL DEFAULT (unixepoch())
);

CREATE TABLE IF NOT EXISTS audit_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source TEXT NOT NULL,
    operation TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id TEXT,
    outcome TEXT NOT NULL,
    reason_code TEXT,
    correlation_id TEXT,
    summary TEXT NOT NULL,
    metadata_json TEXT,
    expires_at INTEGER,
    created_at INTEGER NOT NULL DEFAULT (unixepoch())
);

CREATE INDEX IF NOT EXISTS idx_uplinks_dev_eui ON uplinks (dev_eui);
CREATE INDEX IF NOT EXISTS idx_uplinks_gateway_eui ON uplinks (gateway_eui);
CREATE INDEX IF NOT EXISTS idx_device_sessions_dev_addr ON device_sessions (dev_addr);
CREATE INDEX IF NOT EXISTS idx_audit_log_entity ON audit_log (entity_type, entity_id);
CREATE INDEX IF NOT EXISTS idx_audit_log_operation ON audit_log (operation, created_at);
CREATE INDEX IF NOT EXISTS idx_uplinks_expires_at ON uplinks (expires_at) WHERE expires_at IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_audit_log_expires_at ON audit_log (expires_at) WHERE expires_at IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_downlinks_state ON downlinks (state, priority, created_at);
CREATE INDEX IF NOT EXISTS idx_downlinks_scheduled_at ON downlinks (scheduled_at) WHERE scheduled_at IS NOT NULL;