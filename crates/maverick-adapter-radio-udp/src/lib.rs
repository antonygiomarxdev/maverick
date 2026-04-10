//! UDP radio transport adapter: resilient wrapper and minimal downlink sender for the core port.

mod gwmp;
mod limits;
mod resilient;
mod stub;
mod udp_downlink;

pub use gwmp::{parse_push_data, parse_push_data_json, GwmpPacketMeta, GwmpUplinkBatch};
pub use resilient::{
    CircuitStateView, CircuitTransition, ResiliencePolicy, ResilientRadioTransport,
};
pub use stub::UdpRadioStub;
pub use udp_downlink::UdpDownlinkTransport;
