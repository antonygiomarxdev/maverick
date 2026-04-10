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
    updated_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS uplinks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    dev_addr INTEGER NOT NULL,
    f_cnt INTEGER NOT NULL,
    payload BLOB NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_uplinks_id ON uplinks(id);

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
