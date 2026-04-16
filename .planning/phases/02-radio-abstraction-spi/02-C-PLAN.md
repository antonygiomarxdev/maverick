---
phase: 02-radio-abstraction-spi
plan: C
type: execute
wave: 2
depends_on:
  - 02-B-PLAN.md
files_modified:
  - Cargo.toml
  - crates/maverick-adapter-radio-spi/Cargo.toml
  - crates/maverick-adapter-radio-spi/build.rs
  - crates/maverick-adapter-radio-spi/src/lib.rs
  - crates/maverick-runtime-edge/Cargo.toml
  - crates/maverick-runtime-edge/src/main.rs
autonomous: true
requirements:
  - RADIO-01
  - RADIO-02
  - RADIO-03

must_haves:
  truths:
    - "Workspace crate `maverick-adapter-radio-spi` exists and implements `UplinkSource` behind Cargo feature `spi`"
    - "Blocking libloragw / HAL calls run inside `spawn_blocking` (or equivalent) from async `next_batch`"
    - "`maverick-runtime-edge` enables SPI only via feature flag; default CI path does not require concentrator hardware"
    - "When `[radio].backend = spi` and feature enabled, runtime selects SPI source; otherwise clear build-time or runtime error"
  artifacts:
    - path: "crates/maverick-adapter-radio-spi/src/lib.rs"
      provides: "SpiUplinkSource"
      contains: "UplinkSource"
---

<objective>
Add the SX1302/SX1303 SPI ingest path using Semtech libloragw (C) via the in-tree `-sys` / `cc` pattern described in `02-RESEARCH.md` — not a hypothetical crates.io `loragw-hal`.

Purpose: Satisfies RADIO-01/02 and completes RADIO-03 for real hardware while keeping x86 developer builds unblocked.
</objective>

<execution_context>
@.planning/phases/02-radio-abstraction-spi/02-RESEARCH.md
@.planning/phases/02-radio-abstraction-spi/02-CONTEXT.md
</execution_context>

<tasks>

<task type="auto">
  <name>Task C-1: Crate scaffold + feature gate</name>
  <description>
    - Add `maverick-adapter-radio-spi` to workspace `members`.
    - Feature `spi` on the crate and optional dependency on `cc` / `bindgen` only when needed per research notes.
    - Document in crate README that ARM cross-build requires sysroot headers (align with `.github/workflows/release.yml` patterns).
  </description>
</task>

<task type="auto">
  <name>Task C-2: libloragw integration</name>
  <description>
    - Vendor or submodule strategy per RESEARCH.md (minimal viable: compile only RX paths needed for uplink).
    - FFI bindings translating `lgw_receive` (or equivalent) packet structs into `UplinkObservation` (fields may be partial — document gaps).
    - All `unsafe` blocks isolated and documented; no panics on hot path.
  </description>
</task>

<task type="auto">
  <name>Task C-3: Runtime composition</name>
  <description>
    - `maverick-runtime-edge` optional dependency on `maverick-adapter-radio-spi` behind `spi` feature.
    - Load `LnsConfigDocument` (or runtime config mirror), select `SpiUplinkSource` when configured.
    - Bind address / SPI path from config — constants for defaults in one place (no magic strings in multiple files).
  </description>
</task>

</tasks>
