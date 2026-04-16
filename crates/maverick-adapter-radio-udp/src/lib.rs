//! UDP radio transport adapter: resilient wrapper and minimal downlink sender for the core port.

mod gwmp;
mod gwmp_udp_uplink_source;
mod limits;
mod resilient;
mod stub;
mod udp_downlink;
mod uplink_ingress;

pub use gwmp::{parse_push_data, parse_push_data_json, GwmpPacketMeta, GwmpUplinkBatch};
pub use gwmp_udp_uplink_source::GwmpUdpUplinkSource;
pub use resilient::{
    CircuitStateView, CircuitTransition, ResiliencePolicy, ResilientRadioTransport,
};
pub use stub::UdpRadioStub;
pub use udp_downlink::UdpDownlinkTransport;
pub use uplink_ingress::GwmpUdpIngressBackend;
