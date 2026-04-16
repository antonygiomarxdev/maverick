---
phase: 02-radio-abstraction-spi
plan: B
type: execute
wave: 1
depends_on:
  - 02-A-PLAN.md
files_modified:
  - crates/maverick-adapter-radio-udp/src/lib.rs
  - crates/maverick-adapter-radio-udp/src/gwmp_udp_uplink_source.rs
  - crates/maverick-runtime-edge/src/ingest/gwmp_loop.rs
  - crates/maverick-runtime-edge/src/main.rs
autonomous: true
requirements:
  - RELI-05
  - RADIO-03

must_haves:
  truths:
    - "`GwmpUdpUplinkSource` implements `UplinkSource` using UDP recv + `parse_push_data`"
    - "`run_radio_ingest_once` and `run_radio_ingest_supervised` drive ingestion via `next_batch()` rather than inline `recv_from` + parse"
    - "Idle/read timeout yields empty batch (or same observable behavior as today) without counting as fatal error"
  artifacts:
    - path: "crates/maverick-adapter-radio-udp/src/gwmp_udp_uplink_source.rs"
      provides: "UDP UplinkSource"
      contains: "impl UplinkSource"
    - path: "crates/maverick-runtime-edge/src/ingest/gwmp_loop.rs"
      provides: "Radio-agnostic ingest loop"
      contains: "next_batch"
---

<objective>
Implement the UDP path as the first `UplinkSource` adapter and refactor the edge ingest loop to depend only on the port trait for receiving batches.

Purpose: Delivers RELI-05 for the default/production UDP path before SPI complexity; keeps GWMP parsing centralized in `maverick-adapter-radio-udp`.
</objective>

<execution_context>
@.planning/phases/02-radio-abstraction-spi/02-CONTEXT.md
@crates/maverick-adapter-radio-udp/src/gwmp.rs
</execution_context>

<tasks>

<task type="auto">
  <name>Task B-1: `GwmpUdpUplinkSource` struct</name>
  <description>
    - Owns `Arc<tokio::net::UdpSocket>` (or equivalent) + recv buffer size + read timeout duration (named constants).
    - `next_batch`: `timeout(recv_from)` → on success parse with `parse_push_data`; on timeout return `Ok(vec![])`; on parse error map to `AppResult` per existing ingest error semantics.
    - Constructor takes bind address string (reuse patterns from current `gwmp_loop`).
  </description>
</task>

<task type="auto">
  <name>Task B-2: Refactor `gwmp_loop.rs`</name>
  <description>
    - Replace direct `socket.recv_from` + `parse_push_data` loops with `while` / supervised loop calling `source.next_batch().await`.
    - Preserve JSON counter outputs and tracing (`GwmpUdpIngressBackend` logging may remain for identity; do not duplicate parse paths).
    - Keep `IngestUplink` invocation path unchanged after batch retrieval.
  </description>
</task>

<task type="auto">
  <name>Task B-3: Wire default UDP backend from config (stub SPI selection)</name>
  <description>
    - When `radio` is None or backend UDP: construct `GwmpUdpUplinkSource`.
    - If backend SPI: either return clear error at startup (“SPI backend not built”) until Plan C lands, or cfg-gate — prefer single codepath that fails fast with actionable message.
  </description>
</task>

</tasks>
