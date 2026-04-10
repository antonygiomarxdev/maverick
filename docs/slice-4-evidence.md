# Slice 4 evidence — transport realism (GWMP ingest + circuit transitions)

Date: 2026-04-10

## Goal

Move transport from probe-only behavior to a realistic inbound path by parsing Semtech GWMP `PUSH_DATA` payloads into `UplinkObservation`, wiring one-shot runtime ingest through core use cases, and improving circuit-breaker observability with half-open transitions.

## What was implemented

| Area | Detail |
|------|--------|
| GWMP parser model | Added `gwmp.rs` in `maverick-adapter-radio-udp` with `GwmpPacketMeta`, `GwmpUplinkBatch`, `parse_push_data`, and `parse_push_data_json`. |
| Payload mapping | Parser decodes `rxpk.data` (base64), extracts `DevAddr` / `FCnt` / `FPort`, maps RF metadata (`rssi`, `lsnr`) into `UplinkObservation`, infers region from frequency with EU868 fallback. |
| Runtime inbound wiring | Added `maverick-edge radio ingest-once --bind --timeout-ms` and `radio ingest-loop --bind --read-timeout-ms --max-messages` for gateway operation. Both execute `IngestUplink` via `SqlitePersistence` repositories + `LoRaWAN10xClassA` protocol module. |
| Circuit observability | `ResilientRadioTransport` now exposes `circuit_state()` and `last_transition()`, and records transitions (`Closed/Open/HalfOpen`) with reason labels. |
| Half-open behavior | After open window elapses, one trial request is allowed; success closes circuit, failure re-opens according to threshold policy. |

## Tests

| Test | Location | Notes |
|------|----------|--------|
| GWMP JSON to uplink observation | `maverick-adapter-radio-udp::gwmp::tests` | Verifies parse path and key uplink fields. |
| GWMP malformed + burst parse | `maverick-adapter-radio-udp::gwmp::tests` | Verifies invalid JSON handling and multi-`rxpk` batches. |
| Half-open closes on success | `maverick-adapter-radio-udp::resilient::tests` | Failing-first transport recovers after open window. |
| Parse failure continuity | `maverick-integration-tests/tests/radio_transport_resilience.rs` | Malformed packet returns `InvalidInput` without panic. |
| Recovery after circuit open | `maverick-integration-tests/tests/radio_transport_resilience.rs` | Circuit transitions back to `Closed` after successful post-open trial. |

## Verification commands (executed)

```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --workspace
```

## Residual risks / follow-ups

- Parser supports `PUSH_DATA` uplink extraction, but **full Semtech gateway protocol flow** (ACK/control path, richer metadata variants, robust packet validation) is not complete.
- `radio ingest-loop` runs as supervised bounded loop (by `--max-messages`), but a full daemon/service lifecycle with signal handling and backpressure telemetry remains future work.
- LoRaWAN field extraction is best-effort for current tests; deeper MAC parsing/decryption belongs in protocol capabilities and future slices.
