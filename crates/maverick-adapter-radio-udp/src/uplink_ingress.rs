//! GWMP UDP uplink ingress backend (composition root selects this implementation).

use maverick_core::ports::{UplinkBackendKind, UplinkIngressBackend};

/// GWMP-over-UDP ingress (Semtech packet forwarder style).
#[derive(Debug, Default, Clone, Copy)]
pub struct GwmpUdpIngressBackend;

impl UplinkIngressBackend for GwmpUdpIngressBackend {
    fn kind(&self) -> UplinkBackendKind {
        UplinkBackendKind::GwmpUdp
    }

    fn id(&self) -> &'static str {
        "gwmp_udp"
    }
}
