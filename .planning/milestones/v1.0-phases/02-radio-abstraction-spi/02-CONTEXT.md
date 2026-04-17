# Phase 2: Radio Abstraction & SPI - Context

**Gathered:** 2026-04-16
**Status:** Ready for planning

<domain>
## Phase Boundary

Make the ingest loop radio-agnostic by introducing an `UplinkSource` port trait, then implement a concrete SPI adapter for SX1302/SX1303 that allows Maverick to ingest frames directly without a Semtech packet forwarder. The existing GWMP/UDP path remains fully functional and becomes one implementation of `UplinkSource`. Radio backend is selectable via config.

Downlink over SPI is NOT in scope for this phase (Phase 3 handles downlink).

</domain>

<decisions>
## Implementation Decisions

### UplinkSource port trait (RADIO-01)
- **D-01:** New trait `UplinkSource` in `maverick-core::ports`, using `async-trait` consistent with all other port traits.
- **D-02:** API shape: `async fn next_batch(&self) -> AppResult<Vec<UplinkObservation>>` — one blocking-style call that returns a batch (empty vec on timeout/idle is OK).
- **D-03:** Stream-based API rejected — more complex, harder to test, harder to wrap blocking SPI HAL calls.
- **D-04:** `GwmpUdpUplinkSource` in `maverick-adapter-radio-udp` wraps existing UDP recv + `parse_push_data` logic, implementing `UplinkSource`.
- **D-05:** The ingest loop in `gwmp_loop.rs` is refactored to call `source.next_batch()` instead of raw socket operations — ingest-loop code becomes backend-agnostic.

### SPI adapter library (RADIO-02)
- **D-06:** Use `loragw-hal` crate (C FFI bindings to Semtech's libloragw) — proven real-hardware compatibility with SX1302/SX1303, supports RAK Pi HAT.
- **D-07:** Pure-Rust SPI/register implementation rejected — implementing the full SX1302 protocol from scratch is out of scope for v1.
- **D-08:** New crate `maverick-adapter-radio-spi` wraps `loragw-hal`, implements `UplinkSource`.

### SPI Cargo feature flag (build isolation)
- **D-09:** The `maverick-adapter-radio-spi` crate is gated behind a `spi` feature flag in `maverick-runtime-edge` — x86 dev boxes and CI lint jobs skip it; ARM release builds enable it.
- **D-10:** CI cross-compile for armv7/aarch64 must install libloragw sysroot headers when `spi` feature is active.

### Radio backend config (RADIO-03)
- **D-11:** New `[radio]` section in `lns-config.toml`:
  ```toml
  [radio]
  backend = "spi"        # or "udp" (default when section absent)
  spi_path = "/dev/spidev0.0"  # required when backend = "spi"
  ```
- **D-12:** When `[radio]` section is absent, backend defaults to `"udp"` — all existing `lns-config.toml` files continue to work unchanged (no breaking change).
- **D-13:** `LnsConfigDocument` gains an optional `radio: Option<RadioConfig>` field with `#[serde(default)]`.

### Hardware compatibility registry (RADIO-04)
- **D-14:** Registry ships as `hardware-registry.toml` bundled in the release archive (not compiled in) so community can extend without code changes.
- **D-15:** Each entry records: `board_name`, `arch` (armv7/aarch64), `spi_device`, `concentrator_model` (sx1302/sx1303), `notes`.
- **D-16:** RAK Pi HAT (RAK2287/RAK5146) is the initial verified entry.
- **D-17:** Registry is read-only documentation — runtime does not parse it; operators reference it manually.

### Claude's Discretion
- Exact `loragw-hal` version and initialization sequence
- Exact `next_batch` idle timeout value for UDP adapter
- File path for `hardware-registry.toml` in release layout

</decisions>

<specifics>
## Specific Ideas

- The ingest loop must remain unchanged when backend switches (RADIO-01 success criterion 1).
- SPI path must be configurable (not hardcoded `/dev/spidev0.0`) because HAT variants differ.
- UDP path must remain fully functional for dev, CI, and simulator use after refactor (RADIO-03 success criterion 3).

</specifics>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase requirements
- `.planning/REQUIREMENTS.md` §RADIO-01 through RADIO-04, RELI-05 — full requirement text and acceptance criteria
- `.planning/ROADMAP.md` §Phase 2 — success criteria (5 items) and dependency on Phase 1

### Existing radio adapter (to refactor)
- `crates/maverick-adapter-radio-udp/src/uplink_ingress.rs` — current `GwmpUdpIngressBackend` (identity only; no data production — will grow to implement `UplinkSource`)
- `crates/maverick-adapter-radio-udp/src/gwmp.rs` — `parse_push_data`, `GwmpUplinkBatch`, `UplinkObservation` construction
- `crates/maverick-runtime-edge/src/ingest/gwmp_loop.rs` — current hardcoded UDP loop (target of refactor)

### Core port traits (patterns to follow)
- `crates/maverick-core/src/ports/uplink_ingress.rs` — existing `UplinkIngressBackend` (identity trait; `UplinkSource` goes alongside this)
- `crates/maverick-core/src/ports/mod.rs` — port re-exports (add `UplinkSource` here)
- `crates/maverick-core/src/ports/session_repository.rs` — example async-trait port pattern to replicate

### Config schema (for backward-compatible radio section addition)
- `crates/maverick-core/src/lns_config.rs` — `LnsConfigDocument` (add `radio: Option<RadioConfig>` with `#[serde(default)]`)

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `parse_push_data()` in `gwmp.rs`: already produces `Vec<UplinkObservation>` from raw bytes — wrap in `GwmpUdpUplinkSource::next_batch()` with minimal change.
- `ResiliencePolicy` / `ResilientRadioTransport` pattern in `maverick-adapter-radio-udp`: same circuit-breaker wrapping pattern can be applied to SPI reads.
- `async-trait` already in workspace deps — no new dependency needed for `UplinkSource`.

### Established Patterns
- Port traits live in `maverick-core::ports`, implementations in adapter crates — `UplinkSource` follows this exactly.
- Blocking I/O wrapped with `tokio::task::spawn_blocking` (see SQLite adapter `run_blocking` helper) — SPI HAL calls will need the same.
- `#[serde(default)]` on optional config fields to preserve backward compatibility — used throughout `lns_config.rs`.

### Integration Points
- `gwmp_loop.rs` `run_radio_ingest_supervised` is the primary refactor target — replace raw socket ops with `source.next_batch()`.
- Runtime composition root (`gwmp_loop.rs`) selects which `UplinkSource` impl to instantiate based on config.
- `UplinkBackendKind` enum in `uplink_ingress.rs` needs a `Spi` variant alongside `GwmpUdp`.

</code_context>

<deferred>
## Deferred Ideas

- SPI downlink (TX) — Phase 3 (Class A Downlink)
- Automatic hardware detection (board auto-probe to select spi_path) — Phase 5 TUI has hardware probe; cross-link there
- USB concentrator adapters (SX1302 USB dongles) — community extension, out of v1 scope

</deferred>

---

*Phase: 02-radio-abstraction-spi*
*Context gathered: 2026-04-16*
