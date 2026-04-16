# Phase 1: Protocol Correctness — Context

**Gathered:** 2026-04-16
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 1 makes Maverick a real LNS: every accepted uplink is cryptographically verified (MIC), frame-counted correctly (32-bit FCnt), free of duplicates, and backed by reliable SQLite operations. No radio abstraction, no downlink, no extension IPC — those are later phases. This phase is purely about correctness and reliability of the ingest pipeline.

Requirements in scope: CORE-01, CORE-02, PROT-01, PROT-02, PROT-03, PROT-04, PROT-05, PROT-06, RELI-01, RELI-02, SEC-01

</domain>

<decisions>
## Implementation Decisions

### Session Keys Architecture
- **D-01:** `NwkSKey` and `AppSKey` are added directly to `SessionSnapshot` in `maverick-domain` as `[u8; 16]` fields — single async query in the hot path, no second port needed.
- **D-02:** No separate `KeyRepository` port. `SessionRepository::get_by_dev_addr` returns the full session including keys.
- **D-03:** SQLite schema: add `nwk_s_key BLOB NOT NULL` and `app_s_key BLOB NOT NULL` columns to the sessions table. Migration required for existing rows (no existing users — clean break acceptable).
- **Rationale:** Project has no production users. Do it right from the start. One query per uplink in the hot path, domain model carries what the domain actually needs.

### MIC Verification Placement
- **D-04:** MIC verification happens in `IngestUplink::execute` after `session.get_by_dev_addr` — not inside `ProtocolCapability::validate_uplink`. The protocol module is stateless; keys come from session state.
- **D-05:** MIC computation uses RustCrypto `aes 0.8.x` + `cmac 0.7.x`. AES-128 CMAC over the LoRaWAN B0 block (standard 1.0.x MIC construction). Frames with invalid MIC are rejected with `AppError::Domain("mic_invalid")` and audited.
- **D-06:** MIC requires full 32-bit FCnt in the B0 block. Therefore FCnt 32-bit reconstruction (D-08) must happen BEFORE MIC verification in the execution order.

### FCnt 32-bit Reconstruction
- **D-07:** FCnt reconstruction lives in `ProtocolCapability::validate_uplink` (or a helper called from there), NOT in the UDP parser. The parser passes the raw 16-bit wire value as `u16`; reconstruction uses `extend_fcnt(wire_u16: u16, session_fcnt: u32) -> u32`.
- **D-08:** Algorithm: `extended = (session_fcnt & 0xFFFF_0000) | wire_u16 as u32`. If `extended < session_fcnt` and `session_fcnt - extended > 32768`, add `0x1_0000` to handle rollover. Accept if `extended > session_fcnt` (strict monotonic per spec).
- **D-09:** `UplinkObservation.f_cnt` field type changes from `u32` to `u16` to accurately represent the wire value. Reconstruction produces the `u32` that flows through to persistence.

### Duplicate Frame Detection
- **D-10:** Dedup is SQLite-backed (not in-memory). Before persisting an uplink, query `uplinks` table for `(dev_addr, f_cnt)` within a configurable time window (default: 30 seconds). If found, discard silently and return `Ok(())` — no error, no audit spam.
- **D-11:** Rationale: in-memory dedup is lost on restart. Since core value is "never lose data," the dedup state must also survive restarts. SQLite query in hot path is acceptable on local hardware.
- **D-12:** Dedup key: `(dev_addr, f_cnt, received_at_ms)`. Window: 30 seconds. Configurable via `lns-config.toml`.

### AppSKey Payload Decryption
- **D-13:** Payload decryption (AES-128 CTR, LoRaWAN FRMPayload) happens in `IngestUplink::execute` after MIC passes. Both raw (encrypted) payload and decrypted payload are stored in SQLite — raw for auditability, decrypted for application use.
- **D-14:** `UplinkRecord` gets a `payload_decrypted: Option<Vec<u8>>` field. `None` if decryption fails (logged as warning, not error — uplink still persisted with raw payload).

### Reliability: Mutex Poison
- **D-15:** Comprehensive audit — ALL `.expect()` calls inside any `Mutex<Connection>` lock scope in `maverick-adapter-persistence-sqlite` are replaced with `?`-propagation returning `AppError::Infrastructure`.
- **D-16:** Specifically: lines 288, 295-296, 312-313, 317, 327, 332, 382, 399-400 in `lns_ops.rs`. All `parse_hex_*` calls must return `Result`, not panic.
- **D-17:** `PoisonError` recovery: if `Mutex::lock()` returns a poison error, log at `tracing::error!` level and return `AppError::Infrastructure("mutex_poisoned")` — do not attempt to use the poisoned guard.

### Reliability: Clean Shutdown
- **D-18:** `std::process::exit()` is removed from all async CLI handler paths. Instead, handlers return `anyhow::Result<()>` (or equivalent) and the `main()` function maps the result to an exit code via `std::process::exit`.
- **D-19:** Before process exit (in `main()`), explicitly drop all `Arc<SqlitePersistence>` instances to trigger WAL checkpoint. If drop does not trigger checkpoint (no `Drop` impl), add explicit `SqlitePersistence::close()` method that calls `PRAGMA wal_checkpoint(TRUNCATE)`.
- **D-20:** Scope: only CLI handler paths that currently call `process::exit` directly. Not a full application lifecycle rewrite.

### Security: UDP Bind Default
- **D-21:** Default UDP bind address changes from `0.0.0.0:17000` to `127.0.0.1:17000`. No backward compat required — project has no production users.
- **D-22:** Bind address remains configurable via CLI flag and `lns-config.toml`. Operator can explicitly set `0.0.0.0` if they need external packet forwarder connectivity.

### Region Inference Fix
- **D-23:** `infer_region()` in `gwmp.rs` — fix the AU915 and AS923 match arms to use non-overlapping frequency ranges that don't shadow US915. Exact frequency boundaries from LoRaWAN Regional Parameters spec.

### Offline-First / Core Constraints
- **D-24:** `maverick-edge` makes zero external HTTP/DNS calls — no telemetry, no update checks, no cloud calls. Verified by code review in this phase.
- **D-25:** Every uplink write to SQLite uses synchronous WAL mode — no buffering that could lose data on crash.

### Claude's Discretion
- Exact B0 block construction details (padding, direction byte, etc.) — follow LoRaWAN 1.0.x spec exactly, researcher to confirm exact byte layout
- Whether `UplinkObservation.f_cnt` becomes `u16` or keeps `u32` with a separate `wire_f_cnt: u16` field — planner decides based on impact on existing code
- AES crate feature flags and dependency placement (workspace vs per-crate) — planner decides per existing patterns
- Dedup window exact default (30s is a starting point) — researcher to validate against LoRaWAN timing constraints

</decisions>

<specifics>
## Specific Ideas

- "El proyecto es nuevo como tal, nadie lo usa actualmente" — clean break on any schema/API changes is explicitly approved by user. No migration compatibility required for existing deployments.
- "Nada puede romper o bloquear el LNS" — reliability fixes should be comprehensive, not targeted. If a code path can panic or block the mutex, fix it in this phase.
- "Debemos ser como chirpstack pero mejores" — implement to LoRaWAN spec exactly. No shortcuts on crypto.

</specifics>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### LoRaWAN Spec
- LoRaWAN 1.0.3 spec §4.3.3 — MIC computation (B0 block construction, AES-128 CMAC)
- LoRaWAN 1.0.3 spec §4.3.2 — FRMPayload encryption (AppSKey, AES-128 CTR)
- LoRaWAN Regional Parameters §2.1-2.5 — AU915, AS923, US915 frequency plans

### Existing Codebase
- `crates/maverick-domain/src/session.rs` — `SessionSnapshot` struct (add key fields here)
- `crates/maverick-core/src/use_cases/ingest_uplink.rs` — `IngestUplink::execute` (add MIC + dedup here)
- `crates/maverick-core/src/protocol/lorawan_10x_class_a.rs` — FCnt validation (add 32-bit extension here)
- `crates/maverick-adapter-persistence-sqlite/src/persistence/lns_ops.rs` — `.expect()` calls to fix (lines 288, 295-296, 312-313, 317, 327, 332, 382, 399-400)
- `crates/maverick-adapter-persistence-sqlite/src/schema.sql` — add key columns to sessions table
- `crates/maverick-adapter-radio-udp/src/uplink_ingress.rs` — UDP parser (`f_cnt` type change if applicable)

### Research
- `.planning/research/STACK.md` — RustCrypto crate recommendations (aes 0.8.x + cmac 0.7.x)
- `.planning/research/PITFALLS.md` — FCnt rollover algorithm, MIC pitfalls, Mutex poison details
- `.planning/codebase/CONCERNS.md` — full list of `.expect()` locations and `process::exit` paths

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `LoRaWAN10xClassA::validate_uplink` — existing FCnt check needs extension, not replacement; MIC verification added before FCnt check in `IngestUplink::execute`
- `AppError::Infrastructure` — correct error variant for Mutex/SQLite failures (already exists)
- `AuditSink::emit` — use for MIC rejection events (pattern already established)
- `UplinkRecord` — extend with `payload_decrypted: Option<Vec<u8>>`

### Integration Points
- `IngestUplink::execute` — primary integration point for MIC + dedup
- `ProtocolContext` — already carries `session: Option<&SessionSnapshot>`; adding keys to `SessionSnapshot` makes them available here automatically
- `SqlitePersistence` — dedup query + schema migration land here
- `maverick-runtime-edge/src/commands.rs` — `process::exit` cleanup target

### Patterns to Follow
- `spawn_blocking` for SQLite operations (established in persistence adapter)
- `async_trait` on all port trait impls
- `thiserror` for new error variants
- `tracing::warn!` for protocol rejections (established pattern)

</code_context>
