# Phase 10: libloragw SPI Integration — Context

**Gathered:** 2026-04-17
**Status:** Ready for planning

<domain>
## Phase Boundary

Integrate Semtech's `libloragw` C library to replace the placeholder `SpiUplinkSource::blocking_poll` with real SX1302/SX1303 concentrator I/O. This is the final piece that enables Maverick to ingest LoRaWAN frames directly from SPI hardware without a packet forwarder. The existing UDP adapter remains fully functional for dev/testing.

**Out of scope:** SPI TX/downlink (Phase 3.1), OTAA join handling (v2)

</domain>

<decisions>
## Prior Decisions (locked from earlier phases)

- **Phase 2:** `UplinkSource` port trait — `async fn next_batch(&self) -> AppResult<Vec<UplinkObservation>>`
- **Phase 2:** In-tree vendored C pattern via `cc` in `build.rs` — `loragw-hal` does NOT exist on crates.io
- **Phase 2:** `bindgen` used once to generate `bindings.rs`, checked into repo — no bindgen at CI time
- **Phase 2:** `Mutex<()>` guard protects libloragw global HAL state — only one thread calls HAL at a time
- **Phase 2:** `build.rs` uses `std::env::var("CARGO_FEATURE_SPI")` — NOT `cfg!(feature = "spi")`
- **Phase 2:** libloragw links against `libm` and `librt`
- **Phase 2:** `lgw_stop()` must be called on `Drop` to avoid EBUSY on next `lgw_start()`
- **Phase 9:** `RadioBackend::Auto` — auto-detects SPI hardware and selects backend at runtime

</decisions>

<gray_areas>
## Gray Areas to Discuss

1. **libloragw source version** — Which version of sx1302_hal to vendor? Track upstream tag vs. commit SHA?

2. **lgw_start failure recovery** — If `lgw_start()` fails (device busy, permissions), should SPI adapter fall back to UDP or fail fast?

3. **Multi-board support in lgw_board_setconf** — RAK2287 (SX1302) vs RAK5146 (SX1303) may need different board configs. Hardcode RAK defaults or make configurable?

4. **Pre-generated bindings regeneration** — When `bindings.rs` needs regeneration (header change), what's the procedure? Generate on ARM target or use `--target` flag?

5. **lgw_receive timeout behavior** — What timeout interval to use for `lgw_receive()` blocking poll? Configurable or fixed?

6. **SPI TX / downlink path** — Phase 3.1 handles Class A TX. Should the Phase 10 integration include the TX HAL functions (lgw_send), or just RX?

</gray_areas>

<codebase_context>
## Existing Code Insights

### Phase 2 Research (authoritative)
- File: `.planning/phases/02-radio-abstraction-spi/02-RESEARCH.md`
- In-tree vendored C pattern: vendor libloragw sources in `maverick-adapter-radio-spi/libloragw/`
- `build.rs` compiles C sources via `cc::Build` when `CARGO_FEATURE_SPI` is set
- Key functions: `lgw_start()`, `lgw_stop()`, `lgw_receive(max_pkt, pkt_data)`
- Key struct: `lgw_pkt_rx_s` — fields: `freq_hz`, `rf_chain`, `modulation`, `datarate`, `rssic`, `snr`, `size`, `payload[256]`
- Anti-pattern: global HAL state without Mutex guard — use `Mutex<()>` serialization
- Anti-pattern: panic inside `spawn_blocking` holding Mutex — poison risk (same as RELI-01)
- Pre-generated `bindings.rs` checked in — do NOT run bindgen at CI time
- `lgw_pkt_rx_s.payload` includes full PHY including MIC (last 4 bytes) — split in Rust conversion layer

### Phase 9 (just completed)
- File: `.planning/phases/09-hardware-auto-detection-spi-enable/09-SUMMARY.md`
- `RadioBackend::Auto` and `RadioIngestSelection::AutoSpi`/`AutoUdp` variants added
- `SpiHardwareHints::probe_spi_hardware()` detects `/dev/spidev*` device nodes
- Auto mode probes at startup, falls back to UDP if no SPI hardware found
- Phase 9 did NOT integrate libloragw — just the auto-detection and selection logic

### Current placeholder (to replace)
- File: `crates/maverick-adapter-radio-spi/src/spi_uplink.rs`
- `blocking_poll()` just calls `std::fs::metadata(path)` to verify device exists, sleeps, returns `UplinkReceive::Idle`
- Contract documented in module docstring: must extract `wire_mic`, `phy_without_mic`, DevAddr, FCnt, FPort, payload from `lgw_pkt_rx_s.payload`
- Test marked `#[ignore = "pending libloragw integration"]`

### UplinkObservation contract (from Phase 1)
- From: `crates/maverick-adapter-radio-spi/src/spi_uplink.rs` module docs
- Fields needed: `dev_addr`, `f_cnt` (u16 wire value), `f_port`, `payload`, `wire_mic: [u8; 4]`, `phy_without_mic: Vec<u8>`, `gateway_eui`, `region`, `rssi`, `snr`
- `wire_mic = payload[size-4..size]` — last 4 bytes
- `phy_without_mic = payload[..size-4]` — everything except MIC
- Without `wire_mic` and `phy_without_mic`, MIC verification receives zeros and ALL frames rejected

### Integration points
- `crates/maverick-adapter-radio-spi/src/ingress_identity.rs` — `SpiConcentratorIngressBackend` (likely no change needed)
- `crates/maverick-adapter-radio-spi/src/lib.rs` — exports `SpiUplinkSource` when `spi` feature active
- `crates/maverick-adapter-radio-spi/Cargo.toml` — `spi` feature gate, `cc` build-dependency
- `crates/maverick-runtime-edge/src/radio_ingest_selection.rs` — runtime selection of `SpiUplinkSource` based on config

### CI/lint consideration
- From Phase 2 research: CI lint must NOT use `--all-features` (would activate `spi` on x86 and fail C compilation)
- Current CI lint command may need confirmation: `cargo clippy --all-targets` (no `--all-features`)

</codebase_context>

<specifics>
## Specific Ideas

- Vendor sx1302_hal (https://github.com/lora-net/sx1302_hal) as `libloragw/` submodule in `maverick-adapter-radio-spi`
- `build.rs` compiles `loragw_hal.c`, `loragw_spi.c`, `loragw_reg.c`, `loragw_sx1302.c`, `loragw_sx1302_rx.c`, `loragw_sx1302_timestamp.c`, `loragw_sx125x.c`, `loragw_sx1250.c`, `loragw_aux.c`, `loragw_com.c`
- `bindings.rs` generated once via bindgen, checked into repo
- `SpiUplinkSource` implements `Drop` to call `lgw_stop()`
- `next_batch()` wraps blocking `lgw_receive` in `spawn_blocking` with `Mutex<()>` guard
- Configurable idle timeout (default 100ms) for `lgw_receive` blocking poll
- Hardcoded RAK2287/RAK5146 board defaults for `lgw_board_setconf()` — works for known-supported hardware
- Auto mode (Phase 9) already selects SPI when hardware detected — no additional wiring needed

</specifics>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Prior phase context
- `.planning/phases/02-radio-abstraction-spi/02-RESEARCH.md` — full technical approach (in-tree vendored C, cc build, bindgen, spawn_blocking, Mutex guard, lgw_start/lgw_stop symmetry)
- `.planning/phases/09-hardware-auto-detection-spi-enable/09-SUMMARY.md` — Phase 9 completed, auto mode working
- `.planning/phases/09-hardware-auto-detection-spi-enable/09-CONTEXT.md` — Phase 9 context with gray areas
- `.planning/phases/02-radio-abstraction-spi/02-CONTEXT.md` — Phase 2 decisions (UplinkSource trait, loragw-hal approach, feature flag strategy)

### Existing code (implement)
- `crates/maverick-adapter-radio-spi/src/spi_uplink.rs` — placeholder to replace with real lgw_receive integration
- `crates/maverick-adapter-radio-spi/src/lib.rs` — existing exports, no structural change needed
- `crates/maverick-adapter-radio-spi/Cargo.toml` — add `cc` build-dep, `libloragw/` vendored sources
- `crates/maverick-adapter-radio-spi/build.rs` — compile libloragw C when `CARGO_FEATURE_SPI` set
- `crates/maverick-adapter-radio-spi/src/bindings.rs` — pre-generated FFI bindings (create via bindgen once)
- `crates/maverick-core/src/ports/uplink_ingress.rs` — `UplinkBackendKind::Spi` variant exists, no change needed

### Requirements
- `.planning/REQUIREMENTS.md` §RADIO-01, RADIO-02 — SPI direct SX1302/SX1303, UplinkSource SPI adapter
- `.planning/ROADMAP.md` §Phase 10 — goal and dependency on Phase 9

### Related (do not implement)
- `.planning/phases/03-protocol-security/03-02-SUMMARY.md` — SPI contract documented for future integration
- `.planning/phases/02-radio-abstraction-spi/VERIFICATION.md` — SPI adapter marked as PARTIAL (placeholder)

</canonical_refs>

<deferred>
## Deferred Ideas

- SPI TX/downlink (lgw_send) — Phase 3.1
- OTAA join handling — v2
- Multiple concentrator support — community extension
- USB concentrator adapters — community extension

</deferred>

---

*Phase: 10-libloragw-spi-integration*
*Context gathered: 2026-04-17*