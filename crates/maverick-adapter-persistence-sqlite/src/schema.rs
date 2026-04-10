//! SQLite table and index identifiers plus DDL. Query text is built from `names` where practical.

/// Logical table names (must stay aligned with [`DDL_INIT`]).
pub mod names {
    pub const SESSIONS: &str = "sessions";
    pub const UPLINKS: &str = "uplinks";
    pub const AUDIT_EVENTS: &str = "audit_events";
}

pub mod sessions_columns {
    pub const DEV_ADDR: &str = "dev_addr";
    pub const DEV_EUI: &str = "dev_eui";
    pub const REGION: &str = "region";
    pub const DEVICE_CLASS: &str = "device_class";
    pub const UPLINK_FCNT: &str = "uplink_fcnt";
    pub const DOWNLINK_FCNT: &str = "downlink_fcnt";
    pub const UPDATED_AT_MS: &str = "updated_at_ms";
}

pub mod uplink_columns {
    pub const DEV_ADDR: &str = "dev_addr";
    pub const F_CNT: &str = "f_cnt";
    pub const PAYLOAD: &str = "payload";
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
    use sessions_columns::{DEVICE_CLASS, DEV_ADDR, DEV_EUI, DOWNLINK_FCNT, REGION, UPLINK_FCNT};
    format!(
        "SELECT {DEV_ADDR}, {DEV_EUI}, {REGION}, {DEVICE_CLASS}, {UPLINK_FCNT}, {DOWNLINK_FCNT} \
         FROM {SESSIONS} WHERE {DEV_ADDR} = ?1"
    )
}

pub fn sql_upsert_session() -> String {
    use names::SESSIONS;
    use sessions_columns::{
        DEVICE_CLASS, DEV_ADDR, DEV_EUI, DOWNLINK_FCNT, REGION, UPDATED_AT_MS, UPLINK_FCNT,
    };
    format!(
        r#"INSERT INTO {SESSIONS} ({DEV_ADDR}, {DEV_EUI}, {REGION}, {DEVICE_CLASS}, {UPLINK_FCNT}, {DOWNLINK_FCNT}, {UPDATED_AT_MS})
VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
ON CONFLICT({DEV_ADDR}) DO UPDATE SET
  {DEV_EUI} = excluded.{DEV_EUI},
  {REGION} = excluded.{REGION},
  {DEVICE_CLASS} = excluded.{DEVICE_CLASS},
  {UPLINK_FCNT} = excluded.{UPLINK_FCNT},
  {DOWNLINK_FCNT} = excluded.{DOWNLINK_FCNT},
  {UPDATED_AT_MS} = excluded.{UPDATED_AT_MS}"#
    )
}

pub fn sql_insert_uplink() -> String {
    use names::UPLINKS;
    use uplink_columns::{DEV_ADDR, F_CNT, PAYLOAD};
    format!("INSERT INTO {UPLINKS} ({DEV_ADDR}, {F_CNT}, {PAYLOAD}) VALUES (?1, ?2, ?3)")
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
