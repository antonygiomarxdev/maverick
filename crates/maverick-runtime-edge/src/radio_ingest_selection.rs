//! Resolve `[radio]` from `lns-config.toml` and construct the active [`UplinkSource`](maverick_core::ports::UplinkSource).

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use maverick_adapter_radio_udp::GwmpUdpUplinkSource;
use maverick_core::error::AppResult;
use maverick_core::lns_config::{LnsConfigDocument, RadioBackend};
use maverick_core::ports::UplinkSource;

#[cfg(feature = "spi")]
use maverick_adapter_radio_spi::SpiUplinkSource;

/// Effective uplink path after reading declarative LNS config (if present).
#[derive(Debug, Clone)]
pub(crate) enum RadioIngestSelection {
    Udp {
        bind: String,
    },
    Spi {
        spi_path: String,
    },
    /// Auto mode: resolved from config + hardware probe
    AutoSpi {
        spi_path: String,
        probed: bool,
    },
    /// Auto mode: no SPI hardware found, falling back to UDP
    AutoUdp {
        bind: String,
        reason: String,
    },
}

pub(crate) fn resolve_radio_ingest(
    lns_config_path: &Path,
    cli_gwmp_bind: String,
) -> Result<RadioIngestSelection, String> {
    if !lns_config_path.is_file() {
        return Ok(RadioIngestSelection::Udp {
            bind: cli_gwmp_bind,
        });
    }
    let raw = std::fs::read_to_string(lns_config_path)
        .map_err(|e| format!("lns-config read ({}): {e}", lns_config_path.display()))?;
    let doc: LnsConfigDocument =
        toml::from_str(&raw).map_err(|e| format!("lns-config parse: {e}"))?;
    doc.validate()?;
    match &doc.radio {
        None => Ok(RadioIngestSelection::Udp {
            bind: cli_gwmp_bind,
        }),
        Some(r) => match r.backend {
            RadioBackend::Udp => Ok(RadioIngestSelection::Udp {
                bind: cli_gwmp_bind,
            }),
            RadioBackend::Spi => {
                let p = r.spi_path.as_deref().unwrap_or("").trim();
                if p.is_empty() {
                    return Err("lns-config: radio.backend=spi requires radio.spi_path".to_string());
                }
                Ok(RadioIngestSelection::Spi {
                    spi_path: p.to_string(),
                })
            }
            RadioBackend::Auto => {
                let spi_hints = crate::runtime_capabilities::probe_spi_hardware();
                if let Some(ref hints) = spi_hints {
                    if !hints.concentrator_candidates.is_empty() {
                        let cand = &hints.concentrator_candidates[0];
                        return Ok(RadioIngestSelection::AutoSpi {
                            spi_path: cand.spi_path.clone(),
                            probed: true,
                        });
                    }
                }
                Ok(RadioIngestSelection::AutoUdp {
                    bind: cli_gwmp_bind,
                    reason: "radio.backend=auto: no SPI concentrator hardware detected".to_string(),
                })
            }
        },
    }
}

pub(crate) async fn build_uplink_source(
    selection: RadioIngestSelection,
    read_timeout: Duration,
) -> AppResult<Arc<dyn UplinkSource>> {
    match selection {
        RadioIngestSelection::Udp { bind } => {
            let s = GwmpUdpUplinkSource::bind(bind, read_timeout).await?;
            Ok(Arc::new(s))
        }
        #[cfg(feature = "spi")]
        RadioIngestSelection::Spi { spi_path } => {
            let s = SpiUplinkSource::new(spi_path, read_timeout)?;
            Ok(Arc::new(s))
        }
        #[cfg(not(feature = "spi"))]
        RadioIngestSelection::Spi { .. } => Err(maverick_core::error::AppError::Infrastructure(
            "radio.backend=spi requires building maverick-edge with --features spi".to_string(),
        )),
        RadioIngestSelection::AutoSpi { spi_path, .. } => {
            #[cfg(feature = "spi")]
            {
                let s = SpiUplinkSource::new(spi_path, read_timeout)?;
                Ok(Arc::new(s))
            }
            #[cfg(not(feature = "spi"))]
            {
                Err(maverick_core::error::AppError::Infrastructure(
                    "radio.backend=auto (SPI detected) requires building maverick-edge with --features spi"
                        .to_string(),
                ))
            }
        }
        RadioIngestSelection::AutoUdp { bind, reason } => {
            tracing::info!("SPI auto-detect: {reason} — using UDP ingest");
            let s = GwmpUdpUplinkSource::bind(bind, read_timeout).await?;
            Ok(Arc::new(s))
        }
    }
}
