# Phase 9: Hardware Auto-Detection & SPI Enable — Context

**Gathered:** 2026-04-17
**Status:** Ready for planning

<domain>
## Phase Boundary

Automatically detect available radio hardware (SX1302/SX1303 concentrator via SPI) on the host system and enable the direct SPI ingest path in Maverick without requiring manual operator configuration. This removes the manual `[radio]` section setup from `lns-config.toml` and makes Maverick truly plug-and-play for known hardware.

**Out of scope:** libloragw RX integration (Phase 10), SPI TX/downlink (Phase 3.1)

</domain>

<decisions>
## Prior Decisions (from earlier phases)

- **Phase 2:** `UplinkSource` port trait enables radio-backend switching without changing ingest loop code
- **Phase 2:** `RadioIngestSelection::Spi { spi_path }` carries the SPI device path; requires `[radio]` section in config
- **Phase 2:** `loragw-hal` crate (C FFI bindings) is the approach for libloragw; SPI adapter is a **placeholder** until Phase 10
- **Phase 2:** `hardware-registry.toml` ships as operator documentation, not parsed at runtime
- **Phase 2:** SPI path `/dev/spidev0.0` is the known-default for RAK Pi HAT
- **Phase 8:** Hardware testing revealed RAK LoRa HAT was not attached — SPI path exists but concentrator unreachable

</decisions>

<gray_areas>
## Gray Areas to Discuss

1. **SPI auto-detection strategy** — How should Maverick detect if SPI concentrator hardware is present?
   - Probe `/dev/spidev*` device nodes for existence + accessibility?
   - Check for known hardware via DT overlay or sysfs?
   - Reference `hardware-registry.toml` patterns?

2. **Auto-enable vs operator confirmation** — If SPI hardware IS detected, should Maverick auto-switch to SPI mode?
   - Auto-switch if SPI hardware found (operator can override in config)?
   - Always prompt via TUI/setup wizard before switching?

3. **Multiple SPI devices** — How to select when multiple SPI devices exist on the host?
   - Use first accessible device?
   - Match against `hardware-registry.toml` board entries?
   - TUI prompt for operator selection?

4. **SPI probe failure fallback** — If `[radio].backend=spi` is configured but the SPI device is inaccessible?
   - Fall back to UDP automatically?
   - Fail with clear diagnostic message?
   - TUI prompt offering recovery options?

5. **Runtime probe integration point** — Where should auto-detection run?
   - On startup before ingest loop initialization?
   - Via explicit `maverick-edge probe` command?
   - In TUI/setup wizard before first run?

</gray_areas>

<codebase_context>
## Existing Code Insights

### Reusable Assets
- `HardwareCapabilities::probe()` in `probe.rs`: Memory and OS detection; pattern to extend
- `RadioEnvironmentHints::probe()` in `runtime_capabilities.rs`: Systemd and packet-forwarder detection; pattern to extend for SPI
- `RadioIngestSelection` enum in `radio_ingest_selection.rs`: Already handles `Udp` and `Spi` variants
- `resolve_radio_ingest()` in `radio_ingest_selection.rs`: Config-to-selection resolution

### Established Patterns
- Best-effort probe with notes for operator — never fail silently
- Hardware capability snapshots are recomputed on startup / config reload
- Feature-flag gated SPI (`#[cfg(feature = "spi")]`) keeps x86 dev builds working

### Integration Points
- `runtime_capabilities.rs` `RuntimeCapabilityReport::build()` — add SPI hardware detection here
- `radio_ingest_selection.rs` `resolve_radio_ingest()` — could accept a "probe hint" for auto-backend
- TUI setup wizard (`setup_wizard.rs`) — natural place to surface auto-detected hardware
- `probe --summary` / `probe` JSON — should report detected SPI hardware

### SPI Adapter Status
- `maverick-adapter-radio-spi` crate exists with `SpiUplinkSource` placeholder
- `blocking_poll()` in `spi_uplink.rs` just sleeps and returns `Idle` — **real implementation is Phase 10 (libloragw)**
- This phase is about **enabling the SPI path** and **auto-configuration**, not implementing the concentrator I/O

</codebase_context>

<specifics>
## Specific Ideas

- Auto-detection should surface in `maverick-edge probe` output — operator visibility is critical
- If SPI hardware detected but `lns-config.toml` has no `[radio]` section, TUI could offer "Enable SPI?" prompt
- The auto-detected SPI path should be configurable override, not hardcoded — "auto" as spi_path value
- Fallback to UDP when SPI device inaccessible maintains reliability principle

</specifics>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase dependencies
- `.planning/ROADMAP.md` §Phase 9 — goal and dependency on Phase 8
- `.planning/ROADMAP.md` §Phase 10 — libloragw SPI Integration (Phase 10 follows Phase 9)
- `.planning/REQUIREMENTS.md` §RADIO-01 through RADIO-04 — full radio requirements
- `.planning/REQUIREMENTS.md` §CORE-03 — hardware probe on startup
- `.planning/REQUIREMENTS.md` §CORE-04 — hardware compatibility registry

### Prior phase context
- `.planning/phases/02-radio-abstraction-spi/02-CONTEXT.md` — SPI architecture decisions, UplinkSource trait, loragw-hal approach
- `.planning/phases/08-hardware-testing-rak-pi/08-CONTEXT.md` — Phase 8 found RAK HAT not attached

### Existing code (implement)
- `crates/maverick-runtime-edge/src/probe.rs` — `HardwareCapabilities::probe()` pattern to extend
- `crates/maverick-runtime-edge/src/runtime_capabilities.rs` — `RuntimeCapabilityReport`, `RadioEnvironmentHints::probe()` pattern
- `crates/maverick-runtime-edge/src/radio_ingest_selection.rs` — `RadioIngestSelection` enum and `resolve_radio_ingest()` for integration point
- `crates/maverick-adapter-radio-spi/src/spi_uplink.rs` — `SpiUplinkSource` placeholder (real impl Phase 10)
- `docs/hardware-registry.toml` — existing registry to reference

### Config schema
- `crates/maverick-core/src/lns_config.rs` — `RadioConfig`, `RadioBackend` for adding "auto" mode

</canonical_refs>

<deferred>
## Deferred Ideas

- libloragw RX integration (`lgw_receive`) — Phase 10
- SPI TX/downlink — Phase 3.1
- Runtime parsing of `hardware-registry.toml` — future phase (community could contribute)
- USB concentrator adapters — community extension, out of v1 scope

</deferred>

---

*Phase: 09-hardware-auto-detection-spi-enable*
*Context gathered: 2026-04-17*
