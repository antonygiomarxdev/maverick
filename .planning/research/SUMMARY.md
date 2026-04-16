# RESEARCH SUMMARY — Maverick LNS
_Synthesized: 2026-04-16_

---

## TL;DR

- **MIC + FCnt must land together in Phase 1** — MIC B0 block requires full 32-bit FCnt; one atomic change, not two
- **`NwkSKey` must be added to `SessionSnapshot`** before MIC can be implemented — architectural decision that gates everything else
- **Extension IPC = axum HTTP `127.0.0.1:17001` + SSE** — gRPC/D-Bus rejected (cross-compilation incompatible); shared SQLite rejected (schema coupling)
- **Direct SPI radio = FFI bind `libloragw` via `bindgen`** — no pure-Rust HAL for SX1302/SX1303; feature-gate it, it's Phase 4
- **Process supervision = systemd `Restart=always` + `sd-notify`** — no Rust crate needed; fix Mutex poison and `process::exit` first or supervisor is useless

## Stack Recommendations

| Concern | Recommendation | Confidence |
|---------|---------------|------------|
| AES-128 CMAC (MIC) | `aes 0.8.x` + `cmac 0.7.x` (RustCrypto) | HIGH |
| SPI radio (SX1302) | FFI → `libloragw` via `bindgen`, Cargo feature-gated | MEDIUM |
| Process supervision | systemd unit `Restart=always` + `sd-notify 0.4` | HIGH |
| Extension IPC (push) | `axum 0.7` SSE on `127.0.0.1:17001` | HIGH |
| Extension IPC (query) | Same axum server, REST endpoints | HIGH |

## Table Stakes Features (must-have for real LNS)

1. **MIC verification** — AES-128 CMAC over B0 block; requires NwkSKey in session
2. **32-bit FCnt reconstruction** — `extend_fcnt(wire_u16, session_u32)` in `ProtocolCapability::validate_uplink`
3. **AppSKey payload decryption** — AES-128 CTR; AppSKey must be stored per-session
4. **UDP ingest hardening** — bind to loopback or configurable interface, not `0.0.0.0`
5. **Uplink metadata persistence** — RSSI, SNR, gateway EUI, timestamp
6. **Class A downlink scheduling** — RX1/RX2 windows (v1 or v2 — needs decision)

## Recommended Build Order

**Phase 1 — Protocol correctness**
- FCnt 32-bit fix in `ProtocolCapability::validate_uplink`
- `NwkSKey` + `AppSKey` in `SessionSnapshot` + schema migration
- MIC verification in `IngestUplink::execute`
- Fix Mutex poison (`.expect()` in `lns_ops.rs`) + `process::exit` cleanup
- Region inference bug fix (AU915/AS923 shadowed)

**Phase 2 — Extension boundary + device registry**
- `UplinkSource` port trait (abstracts UDP + future SPI)
- `DeviceRepository` adapter implementation
- `sync_cursors` table for per-extension watermark
- Axum HTTP server in `maverick-edge` (`127.0.0.1:17001`) with SSE
- UDP bind hardening (configurable, default `127.0.0.1`)

**Phase 3 — Process supervision**
- systemd unit `Restart=always` + `RestartSec=2s`
- `sd-notify` watchdog integration
- Health reporting in supervised ingest loop

**Phase 4 — Direct SPI radio**
- `maverick-adapter-radio-spi` crate, feature-gated
- FFI bind `libloragw` via `bindgen`
- `UplinkSource` impl with `spawn_blocking` + mpsc
- Hardware compatibility registry (TOML, community-contributed)

**Phase 5 — TUI device management**
- Add/edit/remove devices and applications via TUI
- Backed by SQLite via local HTTP API

## Critical Pitfalls to Avoid

| # | Pitfall | Prevention |
|---|---------|-----------|
| 1 | **MIC without NwkSKey** | Add key fields to `SessionSnapshot` in Phase 1; single async query |
| 2 | **Mutex poison from `.expect()` in `lns_ops.rs`** | Replace with `?`-propagation before Phase 1 ships |
| 3 | **`process::exit` bypasses SQLite checkpoint** | Explicit `close()` or return exit codes instead |
| 4 | **Slow extension blocks ingest** | `broadcast` channel capacity ~1000; drop slow receivers |
| 5 | **FCnt rollover misdetection** | Accept if `wire_u16` within +32768 of `(session_fcnt & 0xFFFF)`, reconstruct high 16 bits |

## Open Questions

1. **`NwkSKey` in `SessionSnapshot` vs separate key-fetch port** — research recommends `SessionSnapshot`; needs decision before Phase 1
2. **UDP bind default** — `127.0.0.1` (breaks external packet forwarders) vs configurable with opt-in `0.0.0.0`
3. **Downlink in v1?** — Class A RX windows require sub-second precision; `DownlinkRepository` not implemented; v1 or v2?
4. **`libloragw` cross-compilation** — CI cross-compiles armv7/aarch64 from x86_64; C FFI needs sysroot headers; validate before committing to Phase 4

---
*Sources: STACK.md (296 lines), FEATURES.md (128 lines), ARCHITECTURE.md (302 lines), PITFALLS.md (727 lines)*