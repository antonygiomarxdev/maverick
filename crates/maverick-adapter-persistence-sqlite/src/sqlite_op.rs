//! Stable labels for SQLite error context (avoid scattered magic operation strings).

use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SqliteOperation {
    Open,
    BusyTimeout,
    Schema,
    Exec,
    CountUplinks,
    CountAudit,
    CountSessions,
}

impl Display for SqliteOperation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            SqliteOperation::Open => "open",
            SqliteOperation::BusyTimeout => "busy_timeout",
            SqliteOperation::Schema => "schema",
            SqliteOperation::Exec => "exec",
            SqliteOperation::CountUplinks => "count_uplinks",
            SqliteOperation::CountAudit => "count_audit",
            SqliteOperation::CountSessions => "count_sessions",
        };
        f.write_str(s)
    }
}
