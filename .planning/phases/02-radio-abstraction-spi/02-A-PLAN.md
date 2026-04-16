---
phase: 02-radio-abstraction-spi
plan: A
type: execute
wave: 1
depends_on: []
files_modified:
  - crates/maverick-core/src/ports/uplink_source.rs
  - crates/maverick-core/src/ports/mod.rs
  - crates/maverick-core/src/lns_config.rs
autonomous: true
requirements:
  - RELI-05
  - RADIO-03

must_haves:
  truths:
    - "`UplinkSource` async trait exists in `maverick-core::ports` with `next_batch() -> AppResult<Vec<UplinkObservation>>`"
    - "`LnsConfigDocument` includes `radio: Option<RadioConfig>` with `#[serde(default)]`"
    - "Omitting `[radio]` in TOML deserializes as UDP-compatible default (no backend field required)"
    - "`RadioConfig` validation rejects SPI backend without `spi_path` when present"
  artifacts:
    - path: "crates/maverick-core/src/ports/uplink_source.rs"
      provides: "UplinkSource port trait"
      contains: "async fn next_batch"
    - path: "crates/maverick-core/src/lns_config.rs"
      provides: "Optional [radio] mapping"
      contains: "RadioConfig"
---

<objective>
Establish the hexagonal `UplinkSource` port and backward-compatible `[radio]` configuration so downstream plans can implement UDP and SPI adapters without further core API churn.

Purpose: RELI-05 requires a single ingest abstraction; RADIO-03 requires config-selectable backends without breaking existing `lns-config.toml` files.
</objective>

<execution_context>
@.planning/phases/02-radio-abstraction-spi/02-CONTEXT.md
@.planning/phases/02-radio-abstraction-spi/02-RESEARCH.md
</execution_context>

<tasks>

<task type="auto">
  <name>Task A-1: Add `UplinkSource` trait</name>
  <description>
    - New module `ports/uplink_source.rs`: `#[async_trait] trait UplinkSource: Send + Sync` with `async fn next_batch(&self) -> AppResult<Vec<UplinkObservation>>`.
    - Re-use `UplinkObservation` from `ports::radio_transport` (already core-local).
    - Export from `ports/mod.rs` and ensure docs mention empty vec = idle/timeout is OK.
  </description>
</task>

<task type="auto">
  <name>Task A-2: Extend `LnsConfigDocument` with optional radio</name>
  <description>
    - Add `RadioBackend` enum (`udp` / `spi`, serde snake_case or lowercase per project convention).
    - Add `RadioConfig { backend, spi_path: Option<String> }` with `#[serde(default)]` where appropriate.
    - Add `radio: Option<RadioConfig>` to `LnsConfigDocument` with `#[serde(default)]`.
    - Extend `validate()` to enforce: if `radio.backend == Spi` then `spi_path` is Some(non-empty).
    - Add unit tests for default/absent radio, valid SPI row, invalid SPI missing path.
  </description>
</task>

</tasks>
