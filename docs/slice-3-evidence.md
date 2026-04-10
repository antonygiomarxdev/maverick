# Slice 3 evidence — transport resilience & adapter isolation

Date: 2026-04-10

## Goal

Isolate radio **transport** behind `maverick-core::ports::RadioTransport` with **timeout**, **bounded retry**, **exponential backoff**, and a **circuit breaker**, without adding network dependencies to core. Wire a **minimal UDP downlink** path in the edge CLI for observable behavior.

## What was implemented

| Area | Detail |
|------|--------|
| Resilience wrapper | `ResilientRadioTransport` in `maverick-adapter-radio-udp` wraps `Arc<dyn RadioTransport>`; uses `tokio::time::timeout`, retry + backoff from `ResiliencePolicy`, trips circuit after consecutive post-retry failures; surfaces `AppError::CircuitOpen`. |
| UDP downlink | `UdpDownlinkTransport` binds `0.0.0.0:0`, sends `DownlinkFrame.payload` via `send_to` to configured gateway `SocketAddr`. |
| Stub | `UdpRadioStub` remains for explicit “not configured” wiring; message is a stable infrastructure string. |
| Edge CLI | `maverick-edge radio downlink-probe` composes UDP + resilient wrapper; output JSON uses centralized keys (`edge_json` + `EdgeJsonKey` enum). |
| SQLite SRP | `persistence/` submodule split: `sql`, `busy`, `pruning`, `pressure`, `repos`. |
| VO | `UnixMillis` newtype for persisted SQLite timestamps in adapter `sql` module. |

## Tests

| Test | Location | Notes |
|------|----------|--------|
| Timeout on hung inner transport | `maverick-adapter-radio-udp` `resilient::tests` | Inner uses `pending()`; policy short timeout. |
| Circuit opens after threshold | `maverick-adapter-radio-udp` `resilient::tests` | `AlwaysFail` inner; `AppError::CircuitOpen` on third call. |
| UDP payload to listener | `maverick-adapter-radio-udp` `udp_downlink::tests` | Local ephemeral bind/recv. |
| Resilient + UDP success | `maverick-adapter-radio-udp` `udp_downlink::tests` | Wrapper does not break happy path. |
| Cross-crate composition | `maverick-integration-tests/tests/radio_transport_resilience.rs` | Same as above across crates + stub `Infrastructure` error. |

## Verification commands

```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --workspace
```

On hosts where integration test binaries are blocked by policy, run at least:

```bash
cargo test -p maverick-core
cargo test -p maverick-adapter-radio-udp
```

## Residual risks / follow-ups

- **Semtech GWMP / JSON protocol** is not implemented; payload is raw bytes for boundary testing only.
- **Half-open circuit** is modeled as “open until instant”; no separate half-open probe counter (acceptable for v1 slice).
- **Uplink receive path** and full ingest loop are unchanged; transport resilience applies to outbound/downlink probe and future schedulers.
- **Windows Application Control** may block `maverick-integration-tests` integration binaries (os error 4551); use crate-level tests above for CI on affected machines.
