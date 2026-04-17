//! Uplink ingest wiring (LNS guards + transport loops).

mod downlink;
mod gwmp_loop;
mod lns_guard;

pub(crate) use downlink::{DownlinkScheduler, ReceiveWindow, ScheduledDownlink};
pub(crate) use gwmp_loop::{run_radio_ingest_once, run_radio_ingest_supervised};
