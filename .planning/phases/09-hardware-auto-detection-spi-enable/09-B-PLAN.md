---
phase: 09
plan: B
title: Auto-Enable SPI Logic
type: execute
wave: 1
depends_on:
  - 09-A
autonomous: true
files_modified:
  - crates/maverick-core/src/lns_config.rs
  - crates/maverick-runtime-edge/src/radio_ingest_selection.rs
requirements_addressed:
  - CORE-03
  - RADIO-03
---

<objective>
Add "auto" mode to RadioBackend that probes for SPI hardware and auto-enables SPI ingest when concentrator hardware is detected, without requiring manual [radio] config. Fall back to UDP when no SPI hardware found.
</objective>

<tasks>

<task type="execute">
<read_first>
- crates/maverick-core/src/lns_config.rs
- crates/maverick-runtime-edge/src/radio_ingest_selection.rs
</read_first>
<action>
In `crates/maverick-core/src/lns_config.rs`:

1. Add `Auto` variant to `RadioBackend`:
```rust
/// Ingest path: Semtech GWMP/UDP (default) or direct SPI concentrator (Phase 2+).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RadioBackend {
    Udp,
    Spi,
    /// Probe for SPI hardware and auto-select SPI if concentrator detected, otherwise UDP.
    Auto,
}
```

2. Update `RadioConfig::validate()` in lns_config.rs to handle Auto variant:
```rust
if let Some(ref radio) = self.radio {
    match radio.backend {
        RadioBackend::Spi => {
            let path = radio.spi_path.as_deref().unwrap_or("").trim();
            if path.is_empty() {
                return Err(
                    "radio.backend = spi requires non-empty radio.spi_path (e.g. /dev/spidev0.0)"
                        .to_string(),
                );
            }
        }
        RadioBackend::Auto => {
            // Auto mode probes at runtime - no spi_path required in config
        }
        RadioBackend::Udp => {}
    }
}
```

3. Update `RadioConfig::validate()` at line 156-169 to add Auto case:
```rust
match radio.backend {
    RadioBackend::Spi => {
        let path = radio.spi_path.as_deref().unwrap_or("").trim();
        if path.is_empty() {
            return Err(
                "radio.backend = spi requires non-empty radio.spi_path (e.g. /dev/spidev0.0)"
                    .to_string(),
            );
        }
    }
    RadioBackend::Auto => {
        // Auto mode: spi_path is optional, will be probed at runtime
    }
    RadioBackend::Udp => {}
}
```
</action>
<acceptance_criteria>
- RadioBackend::Auto serializes/deserializes correctly as "auto"
- RadioBackend::Auto does not require spi_path in config
- Code compiles with cargo build
- Existing tests pass
</acceptance_criteria>
<verify>
cargo build --package maverick-core 2>&1 | grep -E "error|warning:" || echo "BUILD OK"
cargo test --package maverick-core 2>&1 | tail -20
</verify>
</task>

<task type="execute">
<read_first>
- crates/maverick-runtime-edge/src/radio_ingest_selection.rs
- crates/maverick-runtime-edge/src/runtime_capabilities.rs
</read_first>
<action>
In `crates/maverick-runtime-edge/src/radio_ingest_selection.rs`:

1. Update `RadioIngestSelection` enum to include probe hint:
```rust
/// Effective uplink path after reading declarative LNS config (if present).
#[derive(Debug, Clone)]
pub enum RadioIngestSelection {
    Udp { bind: String },
    Spi { spi_path: String },
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
```

2. Modify `resolve_radio_ingest` to accept optional `SpiHardwareHints`:
```rust
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
                // Auto mode: probe for SPI hardware
                let spi_hints = crate::runtime_capabilities::probe_spi_hardware();
                if let Some(ref hints) = spi_hints {
                    if !hints.concentrator_candidates.is_empty() {
                        // Found concentrator - use first candidate
                        let cand = &hints.concentrator_candidates[0];
                        return Ok(RadioIngestSelection::AutoSpi {
                            spi_path: cand.spi_path.clone(),
                            probed: true,
                        });
                    }
                }
                // No SPI hardware found - fall back to UDP
                Ok(RadioIngestSelection::AutoUdp {
                    bind: cli_gwmp_bind,
                    reason: "radio.backend=auto: no SPI concentrator hardware detected".to_string(),
                })
            }
        },
    }
}
```

3. Update `build_uplink_source` to handle new variants:
```rust
pub(crate) async fn build_uplink_source(
    selection: RadioIngestSelection,
    read_timeout: Duration,
) -> AppResult<Arc<dyn UplinkSource>> {
    match selection {
        RadioIngestSelection::Udp { bind } => {
            let s = GwmpUdpUplinkSource::bind(bind, read_timeout).await?;
            Ok(Arc::new(s))
        }
        RadioIngestSelection::Spi { spi_path } => {
            #[cfg(feature = "spi")]
            {
                let s = SpiUplinkSource::new(spi_path, read_timeout)?;
                Ok(Arc::new(s))
            }
            #[cfg(not(feature = "spi"))]
            {
                Err(maverick_core::error::AppError::Infrastructure(
                    "radio.backend=spi requires building maverick-edge with --features spi".to_string(),
                ))
            }
        }
        RadioIngestSelection::AutoSpi { spi_path, probed: _ } => {
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
```

4. Add `probe_spi_hardware()` as a public function in `runtime_capabilities.rs` so it can be called from `radio_ingest_selection.rs`:
```rust
/// Probe for SPI concentrator hardware on Linux hosts.
/// Returns Some(SpiHardwareHints) if SPI devices found, None otherwise.
pub fn probe_spi_hardware() -> Option<SpiHardwareHints> {
    // Implementation already in Plan A task 3
    // Make the existing probe_spi_hardware() function public
}
```
</action>
<acceptance_criteria>
- resolve_radio_ingest() correctly handles RadioBackend::Auto
- Auto mode probes SPI hardware and selects SPI path if found
- Auto mode falls back to UDP with log message if no SPI hardware found
- build_uplink_source() handles AutoSpi and AutoUdp variants
- Code compiles with cargo build
</acceptance_criteria>
<verify>
cargo build --package maverick-runtime-edge 2>&1 | grep -E "error|warning:" || echo "BUILD OK"
cargo test --package maverick-runtime-edge 2>&1 | tail -20
</verify>
</task>

</tasks>

<verification>
1. Create lns-config.toml with `radio.backend = "auto"` (no spi_path)
2. Run maverick-edge probe --summary and verify it shows SPI hardware detected
3. Create lns-config.toml with `radio.backend = "auto"` on x86 (no SPI) and verify UDP fallback
4. Check logs show appropriate "SPI auto-detect" message
</verification>

<success_criteria>
- Operator can set radio.backend = "auto" without specifying spi_path
- When SPI hardware detected, SPI ingest path is auto-selected
- When no SPI hardware found, UDP is used with informative log message
- No manual [radio] config required for plug-and-play SPI operation
</success_criteria>
