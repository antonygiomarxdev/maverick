//! SQLite table and index identifiers plus DDL. Query text is built from `names` where practical.

/// Logical table names (must stay aligned with [`DDL_INIT`]).
pub mod names {
    pub const SESSIONS: &str = "sessions";
    pub const UPLINKS: &str = "uplinks";
    pub const AUDIT_EVENTS: &str = "audit_events";
    pub const LNS_APPLICATIONS: &str = "lns_applications";
    pub const LNS_DEVICES: &str = "lns_devices";
    pub const LNS_PENDING: &str = "lns_pending";
    pub const LNS_META: &str = "lns_meta";
}

pub mod sessions_columns {
    pub const DEV_ADDR: &str = "dev_addr";
    pub const DEV_EUI: &str = "dev_eui";
    pub const REGION: &str = "region";
    pub const DEVICE_CLASS: &str = "device_class";
    pub const UPLINK_FCNT: &str = "uplink_fcnt";
    pub const DOWNLINK_FCNT: &str = "downlink_fcnt";
    pub const UPDATED_AT_MS: &str = "updated_at_ms";
    pub const APPLICATION_ID: &str = "application_id";
    pub const NWK_S_KEY: &str = "nwk_s_key";
    pub const APP_S_KEY: &str = "app_s_key";
}

pub mod uplink_columns {
    pub const DEV_ADDR: &str = "dev_addr";
    pub const F_CNT: &str = "f_cnt";
    pub const RECEIVED_AT_MS: &str = "received_at_ms";
    pub const PAYLOAD: &str = "payload";
    pub const PAYLOAD_DECRYPTED: &str = "payload_decrypted";
    pub const APPLICATION_ID: &str = "application_id";
}

pub mod audit_columns {
    pub const SOURCE: &str = "source";
    pub const OPERATION: &str = "operation";
    pub const ENTITY_TYPE: &str = "entity_type";
    pub const ENTITY_ID: &str = "entity_id";
    pub const OUTCOME: &str = "outcome";
    pub const METADATA: &str = "metadata";
    pub const CREATED_AT_MS: &str = "created_at_ms";
}

pub const DDL_INIT: &str = include_str!("schema.sql");

pub fn sql_count_rows(table: &'static str) -> String {
    format!("SELECT COUNT(*) FROM {table}")
}

pub fn sql_select_session_by_dev_addr() -> String {
    use names::SESSIONS;
    use sessions_columns::{
        APPLICATION_ID, APP_S_KEY, DEVICE_CLASS, DEV_ADDR, DEV_EUI, DOWNLINK_FCNT, NWK_S_KEY,
        REGION, UPLINK_FCNT,
    };
    format!(
        "SELECT {DEV_ADDR}, {DEV_EUI}, {REGION}, {DEVICE_CLASS}, {UPLINK_FCNT}, {DOWNLINK_FCNT}, \
         {APPLICATION_ID}, {NWK_S_KEY}, {APP_S_KEY} \
         FROM {SESSIONS} WHERE {DEV_ADDR} = ?1"
    )
}

pub fn sql_upsert_session() -> String {
    use names::SESSIONS;
    use sessions_columns::{
        APPLICATION_ID, APP_S_KEY, DEVICE_CLASS, DEV_ADDR, DEV_EUI, DOWNLINK_FCNT, NWK_S_KEY,
        REGION, UPDATED_AT_MS, UPLINK_FCNT,
    };
    format!(
        r#"INSERT INTO {SESSIONS} ({DEV_ADDR}, {DEV_EUI}, {REGION}, {DEVICE_CLASS}, {UPLINK_FCNT}, {DOWNLINK_FCNT}, {UPDATED_AT_MS}, {APPLICATION_ID}, {NWK_S_KEY}, {APP_S_KEY})
VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
ON CONFLICT({DEV_ADDR}) DO UPDATE SET
  {DEV_EUI} = excluded.{DEV_EUI},
  {REGION} = excluded.{REGION},
  {DEVICE_CLASS} = excluded.{DEVICE_CLASS},
  {UPLINK_FCNT} = excluded.{UPLINK_FCNT},
  {DOWNLINK_FCNT} = excluded.{DOWNLINK_FCNT},
  {UPDATED_AT_MS} = excluded.{UPDATED_AT_MS},
  {APPLICATION_ID} = excluded.{APPLICATION_ID},
  {NWK_S_KEY} = excluded.{NWK_S_KEY},
  {APP_S_KEY} = excluded.{APP_S_KEY}"#
    )
}

pub fn sql_insert_uplink() -> String {
    use names::UPLINKS;
    use uplink_columns::{APPLICATION_ID, DEV_ADDR, F_CNT, PAYLOAD, PAYLOAD_DECRYPTED, RECEIVED_AT_MS};
    format!(
        "INSERT INTO {UPLINKS} ({DEV_ADDR}, {F_CNT}, {RECEIVED_AT_MS}, {PAYLOAD}, {APPLICATION_ID}, {PAYLOAD_DECRYPTED}) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)"
    )
}

pub fn sql_check_uplink_dedup() -> String {
    use names::UPLINKS;
    use uplink_columns::{DEV_ADDR, F_CNT, RECEIVED_AT_MS};
    format!(
        "SELECT COUNT(*) FROM {UPLINKS} WHERE {DEV_ADDR} = ?1 AND {F_CNT} = ?2 AND {RECEIVED_AT_MS} >= ?3"
    )
}

pub fn sql_insert_audit_event() -> String {
    use audit_columns::{
        CREATED_AT_MS, ENTITY_ID, ENTITY_TYPE, METADATA, OPERATION, OUTCOME, SOURCE,
    };
    use names::AUDIT_EVENTS;
    format!(
        r#"INSERT INTO {AUDIT_EVENTS} ({SOURCE}, {OPERATION}, {ENTITY_TYPE}, {ENTITY_ID}, {OUTCOME}, {METADATA}, {CREATED_AT_MS})
VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"#
    )
}

pub fn sql_prune_uplinks_oldest() -> String {
    use names::UPLINKS;
    format!("DELETE FROM {UPLINKS} WHERE id IN (SELECT id FROM {UPLINKS} ORDER BY id ASC LIMIT ?1)")
}

pub fn sql_prune_audit_oldest() -> String {
    use names::AUDIT_EVENTS;
    format!(
        "DELETE FROM {AUDIT_EVENTS} WHERE id IN (SELECT id FROM {AUDIT_EVENTS} ORDER BY id ASC LIMIT ?1)"
    )
}

pub fn sql_prune_sessions_lru() -> String {
    use names::SESSIONS;
    use sessions_columns::{DEV_ADDR, UPDATED_AT_MS};
    format!(
        "DELETE FROM {SESSIONS} WHERE {DEV_ADDR} IN (SELECT {DEV_ADDR} FROM {SESSIONS} ORDER BY {UPDATED_AT_MS} ASC LIMIT ?1)"
    )
}

pub fn sql_hard_trim_uplinks(batch: i64) -> String {
    use names::UPLINKS;
    format!(
        "DELETE FROM {UPLINKS} WHERE id IN (SELECT id FROM {UPLINKS} ORDER BY id ASC LIMIT {batch})"
    )
}

pub fn sql_hard_trim_audit(batch: i64) -> String {
    use names::AUDIT_EVENTS;
    format!(
        "DELETE FROM {AUDIT_EVENTS} WHERE id IN (SELECT id FROM {AUDIT_EVENTS} ORDER BY id ASC LIMIT {batch})"
    )
}

pub fn sql_hard_trim_sessions(batch: i64) -> String {
    use names::SESSIONS;
    use sessions_columns::{DEV_ADDR, UPDATED_AT_MS};
    format!(
        "DELETE FROM {SESSIONS} WHERE {DEV_ADDR} IN (SELECT {DEV_ADDR} FROM {SESSIONS} ORDER BY {UPDATED_AT_MS} ASC LIMIT {batch})"
    )
}
