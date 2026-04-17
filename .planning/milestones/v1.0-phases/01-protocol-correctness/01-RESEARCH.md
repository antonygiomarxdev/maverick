# Phase 1: Protocol Correctness — Research

**Researched:** 2026-04-16
**Domain:** LoRaWAN 1.0.x cryptography (MIC/AES-CMAC, payload decryption/AES-CTR), FCnt 32-bit
reconstruction, SQLite dedup, rusqlite error propagation, clean shutdown in tokio
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** `NwkSKey` and `AppSKey` added directly to `SessionSnapshot` in `maverick-domain` as `[u8; 16]` fields.
- **D-02:** No separate `KeyRepository` port. `SessionRepository::get_by_dev_addr` returns full session including keys.
- **D-03:** SQLite schema: add `nwk_s_key BLOB NOT NULL` and `app_s_key BLOB NOT NULL` to sessions table. Migration via existing `migrate_legacy_columns` pattern.
- **D-04:** MIC verification in `IngestUplink::execute` after session fetch — not inside `ProtocolCapability::validate_uplink`.
- **D-05:** MIC via RustCrypto `aes 0.8.x` + `cmac 0.7.x` (see version note below). AES-128 CMAC over LoRaWAN B0 block. Invalid MIC → `AppError::Domain("mic_invalid")` + audit.
- **D-06:** FCnt 32-bit reconstruction (D-08) must occur BEFORE MIC verification.
- **D-07:** FCnt reconstruction in `ProtocolCapability::validate_uplink` (or helper). Parser passes raw `u16`. `extend_fcnt(wire_u16: u16, session_fcnt: u32) -> u32`.
- **D-08:** Algorithm: `extended = (session_fcnt & 0xFFFF_0000) | wire_u16 as u32`. If `extended < session_fcnt` and `session_fcnt - extended > 32768`, add `0x1_0000`.
- **D-09:** `UplinkObservation.f_cnt` type changes from `u32` to `u16` to represent wire value accurately.
- **D-10:** Dedup is SQLite-backed. Query `uplinks` for `(dev_addr, f_cnt)` within 30-second window before persisting.
- **D-11:** In-memory dedup is lost on restart; SQLite dedup survives.
- **D-12:** Dedup key: `(dev_addr, f_cnt, received_at_ms)`. Window: 30 seconds. Configurable via `lns-config.toml`.
- **D-13:** Payload decryption (AES-128 CTR) in `IngestUplink::execute` after MIC passes. Both raw and decrypted payloads stored.
- **D-14:** `UplinkRecord.payload_decrypted: Option<Vec<u8>>`. `None` if decryption fails (warn, not error).
- **D-15:** All `.expect()` inside `Mutex<Connection>` lock scope replaced with `?`-propagation returning `AppError::Infrastructure`.
- **D-16:** Specific lines: 288, 295-296, 312-313, 317, 327, 332, 382, 399-400 in `lns_ops.rs`.
- **D-17:** `PoisonError` → `tracing::error!` + return `AppError::Infrastructure("mutex_poisoned")`.
- **D-18:** `std::process::exit()` removed from async CLI handler paths. Handlers return `anyhow::Result<()>` (or `Result<(), String>`). `main()` maps to exit code.
- **D-19:** Before process exit: drop `Arc<SqlitePersistence>` or call explicit `SqlitePersistence::close()` with `PRAGMA wal_checkpoint(TRUNCATE)`.
- **D-20:** Scope: CLI handler paths that currently call `process::exit` directly.
- **D-21:** Default UDP bind address changes from `0.0.0.0:17000` to `127.0.0.1:17000`.
- **D-22:** Bind address remains configurable via CLI flag / `lns-config.toml`.
- **D-23:** Fix `infer_region()` AU915 and AS923 match arms to use non-overlapping frequency ranges.
- **D-24:** `maverick-edge` makes zero external HTTP/DNS calls — verified by code review.
- **D-25:** Every uplink write to SQLite uses synchronous WAL mode.

### Claude's Discretion

- Exact B0 block construction details — follow LoRaWAN 1.0.x spec exactly, researcher to confirm exact byte layout.
- Whether `UplinkObservation.f_cnt` becomes `u16` or keeps `u32` with a separate `wire_f_cnt: u16` field.
- AES crate feature flags and dependency placement (workspace vs per-crate).
- Dedup window exact default (30s is a starting point) — validate against LoRaWAN timing constraints.

### Deferred Ideas (OUT OF SCOPE)

- Extension IPC, OTAA, output plugins (v2)
- Downlink RX1/RX2 windows (Phase 3)
- SPI radio adapter (Phase 2)
- Process supervision systemd unit (Phase 4)
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| CORE-01 | Zero external HTTP/DNS calls | Code audit — no HTTP client in workspace deps; verified by `grep` |
| CORE-02 | Every accepted uplink written to SQLite before acknowledgment | Existing `IngestUplink::execute` append-then-upsert pattern; needs WAL verification |
| PROT-01 | MIC (AES-128 CMAC) verification on every uplink | RustCrypto `aes 0.9` + `cmac 0.8` verified; B0 block layout documented below |
| PROT-02 | 32-bit FCnt reconstruction from 16-bit wire value | Algorithm and rollover handling documented with exact code example |
| PROT-03 | NwkSKey + AppSKey stored per session | Schema migration pattern documented; `SessionSnapshot` extension specified |
| PROT-04 | Payload decrypted with AppSKey (AES-128-CTR) and persisted | AES-CTR keystream pattern verified; `UplinkRecord` extension specified |
| PROT-05 | Region inference correctly identifies AU915/AS923 | Exact non-overlapping frequency boundaries documented |
| PROT-06 | Duplicate frame detection | SQLite dedup query + `received_at_ms` column addition specified |
| RELI-01 | Mutex not permanently poisonable | All `.expect()` locations identified; `?`-propagation pattern documented |
| RELI-02 | Clean shutdown checkpoints WAL | WAL checkpoint pattern and `Drop`/explicit close approach documented |
| SEC-01 | UDP bind configurable, default `127.0.0.1:17000` | Exact constant location identified: `cli_constants.rs:DEFAULT_GWMP_BIND_ADDR` |
</phase_requirements>

---

## Summary

Phase 1 transforms Maverick from an open relay into a real LoRaWAN LNS. The work falls into
five streams: (1) cryptographic verification (MIC + payload decryption), (2) FCnt 32-bit
reconstruction, (3) SQLite-backed duplicate detection, (4) reliability hardening
(Mutex/.expect() audit + clean shutdown), and (5) security defaults (UDP bind address + region
inference fix).

All cryptographic primitives are solved by RustCrypto. **Key version finding:** the STACK.md
reference to `aes 0.8.x` and `cmac 0.7.x` is stale — current crates.io versions are
`aes 0.9.0` and `cmac 0.8.0`. Both have been verified to compile together and the trait API
has changed: `new_from_slice` now requires explicit `use cmac::KeyInit` (no longer re-exported
through `Mac`). The AES cipher trait is `BlockCipherEncrypt` (not `BlockEncrypt`) in aes 0.9.

The FCnt and dedup work requires adding a `received_at_ms` column to the `uplinks` table
(currently absent). The existing migration pattern in `sql.rs` handles this cleanly.

The reliability fixes are mechanical but comprehensive: 28+ `process::exit` call sites in
`commands.rs` and `commands/config.rs`, plus 9+ `.expect()` calls inside Mutex lock scope in
`lns_ops.rs` (lines 288, 295-296, 312-313, 317, 327, 332, 382, 399-400).

**Critical note on `panic = "abort"` and Mutex poison:** The release profile uses
`panic = "abort"` (Cargo.toml line 58). In release builds, panics abort immediately — no
unwinding — so Mutex poison is mechanically impossible at runtime. However, dev and test
builds use default unwinding, meaning tests that panic inside a Mutex guard WILL poison it.
The `.expect()` removal is still required to eliminate the panic source itself (not just to
handle poison).

**Primary recommendation:** Implement in dependency order — (1) FCnt u16 type change + 32-bit
extension, (2) schema migration adding `nwk_s_key`/`app_s_key` to sessions + `received_at_ms`
to uplinks, (3) MIC verification using aes 0.9 + cmac 0.8, (4) payload decryption, (5) dedup
query, (6) .expect() audit in lns_ops.rs, (7) process::exit cleanup, (8) UDP bind default +
region fix.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| FCnt 32-bit reconstruction | `maverick-core` (protocol module) | `maverick-adapter-radio-udp` (wire parse) | Spec §4.3.1.5 is a server-side LNS concern; parser provides raw u16 |
| MIC verification | `maverick-core` (use case) | `maverick-domain` (SessionSnapshot carries keys) | Keys are session state; verification is application logic |
| Payload decryption | `maverick-core` (use case) | `maverick-adapter-persistence-sqlite` (stores both payloads) | AppSKey in session; decryption is ingest concern |
| Duplicate detection | `maverick-adapter-persistence-sqlite` | `maverick-core` (use case checks before append) | SQLite-backed per D-10; query lives in adapter |
| Session key storage | `maverick-domain` + `maverick-adapter-persistence-sqlite` | — | Domain model carries keys; SQLite schema stores them |
| UDP bind default | `maverick-runtime-edge` (`cli_constants.rs`) | — | Composition root owns CLI defaults |
| Region inference fix | `maverick-adapter-radio-udp` (`gwmp.rs`) | — | GWMP parser owns frequency-to-region mapping |
| Mutex/.expect() cleanup | `maverick-adapter-persistence-sqlite` | — | All `.expect()` in adapter's `lns_ops.rs` |
| process::exit cleanup | `maverick-runtime-edge` | — | Runtime binary owns CLI command handlers |

---

## Standard Stack

### Crypto (New — not yet in workspace)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `aes` | 0.9.0 | AES-128 block cipher | RustCrypto; no-std; constant-time; cross-compiles armv7/aarch64 |
| `cmac` | 0.8.0 | CMAC wrapper (AES-128 CMAC = LoRaWAN MIC) | RustCrypto; builds on `aes 0.9`; implements `digest::Mac` |

[VERIFIED: crates.io registry — `cargo search aes` returned `aes = "0.9.0"`, `cargo search cmac` returned `cmac = "0.8.0"`. Both crates fetch and compile together successfully. Tested 2026-04-16.]

### Existing Stack (Already in Workspace)

| Library | Version | Purpose | Phase Use |
|---------|---------|---------|-----------|
| `rusqlite` | 0.33.0 | SQLite client | Dedup query; schema migration; key columns |
| `tokio` | 1.51.1 | Async runtime | `spawn_blocking` for SQLite ops |
| `tracing` | 0.1.44 | Structured logging | Warn on MIC reject; error on mutex poison |
| `thiserror` | 1.x | Error types | No new error variants needed; `AppError::Domain` + `AppError::Infrastructure` already exist |

**Version correction from STACK.md:** STACK.md recommends `aes 0.8.x` and `cmac 0.7.x`.
These are outdated. Current: `aes 0.9.0`, `cmac 0.8.0`. API differences documented in
Code Examples section below.

**Dependency placement recommendation:**

Add to `maverick-core/Cargo.toml` (NOT workspace-level) because:
- Only `maverick-core` needs crypto in Phase 1.
- Avoids pulling AES into `maverick-domain` (pure value objects, no I/O/crypto).
- Consistent with hexagonal: crypto is application logic in the core ring.

```toml
# crates/maverick-core/Cargo.toml
[dependencies]
aes = "0.9"
cmac = { version = "0.8", features = [] }
```

No special feature flags needed — both crates compile with standard defaults.

---

## Architecture Patterns

### System Architecture Diagram

```
UDP Packet (GWMP)
      |
      v
[maverick-adapter-radio-udp]
  gwmp.rs::parse_lorawan_payload()
    → strips raw MIC bytes (last 4)
    → returns (DevAddr, wire_fcnt: u16, f_port, payload, raw_phy)
    → infer_region() [FIX: AU915/AS923 arms]
      |
      v
UplinkObservation { f_cnt: u16 (CHANGED), payload: encrypted, ... }
      |
      v
[maverick-core::use_cases::IngestUplink::execute()]
  1. sessions.get_by_dev_addr(dev_addr)  ← returns SessionSnapshot with nwk_s_key, app_s_key (NEW)
  2. protocol.validate_uplink(ctx)        ← FCnt 32-bit extension HERE (CHANGED)
       extend_fcnt(obs.f_cnt, session.uplink_frame_counter)
       MAX_FCNT_GAP check
       region/class checks
       returns reconstructed_fcnt: u32
  3. compute_mic(nwk_s_key, b0_block, phy_without_mic)  ← NEW
       reject if != wire_mic → AuditSink + AppError::Domain("mic_invalid")
  4. check_dedup(dev_addr, reconstructed_fcnt, 30s)     ← NEW
       uplinks.is_duplicate(dev_addr, f_cnt)
       if dup → return Ok(()) silently
  5. decrypt_payload(app_s_key, dev_addr, f_cnt, payload)  ← NEW
       AES-128-CTR; None on failure
  6. uplinks.append(UplinkRecord { f_cnt: reconstructed_u32, payload, payload_decrypted })  ← CHANGED
  7. sessions.upsert(session with updated uplink_frame_counter = reconstructed_fcnt)
  8. audit.emit("success")
```

### Recommended Project Structure

No new crates. All changes are within existing crates:

```
crates/
├── maverick-domain/src/
│   └── session.rs                 # ADD: nwk_s_key: [u8;16], app_s_key: [u8;16]
├── maverick-core/src/
│   ├── protocol/
│   │   └── lorawan_10x_class_a.rs # CHANGE: extend_fcnt helper; MAX_FCNT_GAP check
│   ├── use_cases/
│   │   └── ingest_uplink.rs       # ADD: MIC verify, dedup check, payload decrypt
│   └── ports/
│       ├── radio_transport.rs     # CHANGE: UplinkObservation.f_cnt: u32 → u16
│       └── uplink_repository.rs   # ADD: UplinkRecord.payload_decrypted, is_duplicate port method
├── maverick-adapter-persistence-sqlite/src/
│   ├── schema.sql                 # ADD: nwk_s_key/app_s_key on sessions; received_at_ms on uplinks
│   ├── persistence/
│   │   ├── sql.rs                 # ADD: migrate_sessions_v2 (key columns), migrate_uplinks_v2
│   │   ├── repos.rs               # ADD: is_duplicate query; payload_decrypted in append
│   │   └── lns_ops.rs             # FIX: all .expect() → ? on lines 288,295,296,312,313,317,327,332,382,399,400
│   └── lib.rs or mod.rs           # ADD: SqlitePersistence::close() for WAL checkpoint
├── maverick-adapter-radio-udp/src/
│   └── gwmp.rs                    # FIX: infer_region() AU915/AS923 arm ordering
└── maverick-runtime-edge/src/
    ├── cli_constants.rs            # CHANGE: DEFAULT_GWMP_BIND_ADDR = "127.0.0.1:17000"
    ├── commands.rs                 # FIX: process::exit → return Err; main maps to exit code
    └── commands/config.rs          # FIX: all 25 process::exit sites → return Err
```

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| AES-128 CMAC | custom CMAC implementation | `aes 0.9` + `cmac 0.8` | Constant-time guarantees; NIST-tested; 600+ downloads/day |
| AES-128-CTR keystream | custom CTR loop | `aes 0.9` `BlockCipherEncrypt` | Safe encrypt_block API; avoids endian bugs |
| FCnt rollover math | custom rollover detection | Use the exact algorithm in D-08 | Off-by-one in rollover detection bricks sessions past 65535 uplinks |
| SQLite dedup query | in-memory HashMap | SQL `EXISTS` or `SELECT COUNT` on `uplinks` | Survives process restart; no lock contention |

**Key insight:** The LoRaWAN crypto is 3 pages of spec. The RustCrypto crates implement
it in ~50 lines of verified code. Anything hand-rolled will fail on constant-time
guarantees, endian byte order, or block counter starting index (off-by-one at `i=1`).

---

## Common Pitfalls

### Pitfall 1: Wrong crate version API — `cmac 0.7.x` vs `0.8.x`

**What goes wrong:** STACK.md recommends `aes 0.8.x` + `cmac 0.7.x`. Current crates.io
versions are `aes 0.9.0` + `cmac 0.8.0`. The API changed:

- In `cmac 0.7.x`: `Cmac::<Aes128>::new_from_slice(key)` worked with only `use cmac::Mac`.
- In `cmac 0.8.x`: `new_from_slice` requires `use cmac::KeyInit` explicitly. `Mac` alone is
  insufficient.
- In `aes 0.9.x`: The encrypt trait is `BlockCipherEncrypt` (not `BlockEncrypt`).

**Why it happens:** RustCrypto trait tower refactored `KeyInit` into its own trait in
`crypto-common 0.2.x`.

**How to avoid:** Use exactly the imports shown in Code Examples below.
**Warning signs:** `E0599: method not found in Cmac<Aes128>` at `new_from_slice`.

---

### Pitfall 2: B0 block byte order — DevAddr and FCnt must be little-endian

**What goes wrong:** B0 block bytes 6-9 (DevAddr) and 10-13 (FCnt) must be little-endian.
`DevAddr` is stored internally as `u32` big-endian (network byte order). Using `to_be_bytes()`
on the DevAddr produces incorrect MIC.

**Why it happens:** LoRaWAN spec §4.4 explicitly states LE for B0 fields, but "network byte
order" instinct says big-endian.

**How to avoid:** Always use `dev_addr.0.to_le_bytes()` and `f_cnt.to_le_bytes()` when
building the B0 block.

**Warning signs:** MIC fails for some DevAddr values but not others (when the LE/BE value
coincidentally matches, e.g. `0x01010101`).

---

### Pitfall 3: FCnt AES CTR block counter starts at `i = 1`, NOT `i = 0`

**What goes wrong:** LoRaWAN §4.3.3.2 specifies the `Ai` block counter starts at `i = 1` for
the first 16-byte block. Starting at `i = 0` produces a different keystream → garbage decryption.

**Why it happens:** Common CTR mode implementations start at 0. The LoRaWAN spec is an
explicit exception documented in the pitfalls research.

**How to avoid:** Loop `for i in 1u8..=(block_count as u8)`, not `0..block_count`.
**Warning signs:** Decryption silently "succeeds" but payload is wrong.

---

### Pitfall 4: FCnt rollover — `extend_fcnt` algorithm must handle the `session_fcnt - extended > 32768` case

**What goes wrong:** D-08 algorithm: after computing `extended = (session_fcnt & 0xFFFF_0000) | wire_u16`, if `extended < session_fcnt`, the subtraction `session_fcnt - extended` could underflow if `session_fcnt` is just above a 16-bit boundary (e.g., `session_fcnt = 0x0001_0001`, `wire_u16 = 0x0000`).

**Why it happens:** Unsigned subtraction underflow when `extended` overflows u32 after adding `0x1_0000`.

**How to avoid:** Use saturating or wrapping arithmetic in the rollover check:
```rust
let needs_rollover = extended < session_fcnt
    && session_fcnt.wrapping_sub(extended) > 32768;
```

---

### Pitfall 5: `.expect()` panics in `spawn_blocking` — context matters

**What goes wrong:** `lns_ops.rs` has ~11 `.expect("validated lns config")` calls inside the
closure passed to `run_with_busy_retry` → called from `run_blocking` → `spawn_blocking`. A
panic in `spawn_blocking` does NOT propagate to the calling async task automatically —
`JoinError::is_panic()` is true but the panic is contained. However, the `std::sync::Mutex`
over `Connection` in `Inner` is held at panic time, poisoning it.

**Critical nuance:** `panic = "abort"` in `[profile.release]` means NO unwinding in release
builds — mutex poison is impossible in production. However in debug/test builds (default
`unwind`), any `.expect()` panic in a mutex guard context poisons the mutex for the test
process lifetime. This makes tests non-deterministic if they share a `SqlitePersistence`.

**How to avoid:** Convert `.expect()` to `?` returning `rusqlite::Error` (which `run_with_busy_retry`
already maps to `AppError::Infrastructure`).

---

### Pitfall 6: MIC verification requires raw phy bytes (before the MIC strip)

**What goes wrong:** `parse_lorawan_payload` in `gwmp.rs` strips the last 4 bytes (MIC) and
discards them. The current `UplinkObservation` carries no reference to the raw frame or the
stripped MIC bytes. MIC verification requires:
- The 4 MIC bytes (last 4 bytes of raw PHY payload)
- The payload WITHOUT those 4 bytes (MHDR || FHDR || FPort || FRMPayload)

**How to avoid:** Either (a) store `raw_mic: [u8; 4]` in `UplinkObservation`, or (b) pass
the raw phy bytes through and do the strip + MIC verify together in `IngestUplink::execute`.
Approach (b) is cleaner given D-04 (MIC verification is in the use case, not the protocol module).
The parser should expose `wire_mic: [u8; 4]` and `phy_without_mic: Vec<u8>` in `UplinkObservation`.

---

### Pitfall 7: `process::exit` in async context races with in-flight futures

**What goes wrong:** There are 35 `process::exit` call sites across `commands.rs` and
`commands/config.rs`. These are all in CLI handler functions (not the ingest loop, except
`gwmp_loop.rs:190` and `gwmp_loop.rs:214`). When called from inside an `async fn` scheduled
on tokio, `process::exit` abandons all in-flight `spawn_blocking` tasks — including any
active SQLite writes.

**How to avoid (D-18 pattern):** Convert each command handler's return type from `()` to
`Result<(), String>` (or `anyhow::Result<()>`). Replace every `process::exit(N)` with
`return Err("...".to_string())`. In `main()`, pattern-match the result:
```rust
match dispatch_command(cmd).await {
    Ok(()) => {},
    Err(_) => std::process::exit(1),
}
```
The Tokio runtime drop (when `main` returns) gives in-flight futures a chance to complete.

---

### Pitfall 8: Dedup requires `received_at_ms` column — not yet in `uplinks` table

**What goes wrong:** D-12 specifies dedup key `(dev_addr, f_cnt, received_at_ms)` with a
30-second window. The current `uplinks` schema has NO `received_at_ms` column.

**How to avoid:** Add migration in `sql.rs` (following the existing `migrate_legacy_columns`
pattern):
```rust
fn migrate_uplinks_v2(conn: &mut Connection) -> Result<(), AppError> {
    let _ = conn.execute("ALTER TABLE uplinks ADD COLUMN received_at_ms INTEGER", []);
    Ok(())
}
```
Also add index: `CREATE INDEX IF NOT EXISTS idx_uplinks_dedup ON uplinks(dev_addr, f_cnt, received_at_ms)`.

---

## Code Examples

All examples below are verified against `aes 0.9.0` + `cmac 0.8.0` (compiled and run 2026-04-16).

### MIC Computation: B0 Block Construction

[VERIFIED: compiled and run against aes 0.9.0 + cmac 0.8.0, 2026-04-16]

```rust
// Cargo.toml in maverick-core:
// aes = "0.9"
// cmac = "0.8"

use aes::Aes128;
use cmac::{Cmac, KeyInit, Mac};  // KeyInit is REQUIRED in cmac 0.8 — was not needed in 0.7

/// LoRaWAN 1.0.x §4.4 — B0 block for uplink MIC computation.
/// All multi-byte fields are LITTLE-ENDIAN.
///
/// Byte layout:
/// [0]     0x49
/// [1..5]  0x00 0x00 0x00 0x00  (four reserved zeroes)
/// [5]     dir  (0x00 for uplink, 0x01 for downlink)
/// [6..10] DevAddr LE (4 bytes)
/// [10..14] FCntUp LE (4 bytes — FULL 32-bit reconstructed counter)
/// [14]    0x00
/// [15]    len  (length in bytes of MHDR||FHDR||FPort||FRMPayload, i.e. PHY without trailing MIC)
pub fn build_b0_uplink(dev_addr: u32, f_cnt: u32, phy_len_without_mic: usize) -> [u8; 16] {
    let mut b0 = [0u8; 16];
    b0[0] = 0x49;
    // b0[1..5] = 0x00 already
    b0[5] = 0x00; // uplink direction
    b0[6..10].copy_from_slice(&dev_addr.to_le_bytes());
    b0[10..14].copy_from_slice(&f_cnt.to_le_bytes());
    // b0[14] = 0x00 already
    b0[15] = phy_len_without_mic as u8;
    b0
}

/// Compute AES-128 CMAC over B0 || PHY_without_MIC, return first 4 bytes.
pub fn compute_mic(nwk_s_key: &[u8; 16], b0: &[u8; 16], phy_without_mic: &[u8]) -> [u8; 4] {
    let mut mac = <Cmac<Aes128> as KeyInit>::new_from_slice(nwk_s_key)
        .expect("NwkSKey is always 16 bytes");
    mac.update(b0);
    mac.update(phy_without_mic);
    let full = mac.finalize().into_bytes();
    [full[0], full[1], full[2], full[3]]
}

/// Verify wire MIC against computed MIC. Constant-time comparison via cmac finalize_reset.
pub fn verify_mic(
    nwk_s_key: &[u8; 16],
    b0: &[u8; 16],
    phy_without_mic: &[u8],
    wire_mic: &[u8; 4],
) -> bool {
    let computed = compute_mic(nwk_s_key, b0, phy_without_mic);
    computed == *wire_mic
}
```

---

### AES-128-CTR FRMPayload Decryption

[VERIFIED: compiled and run against aes 0.9.0, 2026-04-16]

```rust
// In maverick-core, requires: aes = "0.9" in Cargo.toml
use aes::Aes128;
use aes::cipher::{BlockCipherEncrypt, KeyInit}; // BlockCipherEncrypt (not BlockEncrypt) in aes 0.9

/// LoRaWAN 1.0.x §4.3.3.2 — AES-128-CTR FRMPayload decryption.
///
/// The Ai block for block i (1-based):
/// [0]     0x01
/// [1..5]  0x00 0x00 0x00 0x00  (four reserved zeroes)
/// [5]     dir  (0x00 = uplink, 0x01 = downlink)
/// [6..10] DevAddr LE
/// [10..14] FCntUp LE (32-bit reconstructed)
/// [14]    0x00
/// [15]    i  (block counter, starts at 1, NOT 0)
///
/// Encryption and decryption are the same operation (XOR with keystream).
pub fn decrypt_frm_payload(
    app_s_key: &[u8; 16],
    dev_addr: u32,
    f_cnt: u32,
    dir: u8, // 0 = uplink
    payload: &[u8],
) -> Vec<u8> {
    if payload.is_empty() {
        return Vec::new();
    }
    let cipher = <Aes128 as KeyInit>::new_from_slice(app_s_key)
        .expect("AppSKey is always 16 bytes");
    let block_count = payload.len().div_ceil(16); // == (len + 15) / 16
    let mut keystream = Vec::with_capacity(block_count * 16);

    for i in 1u8..=(block_count as u8) { // CRITICAL: starts at 1, not 0
        let mut ai = [0u8; 16];
        ai[0] = 0x01;
        ai[5] = dir;
        ai[6..10].copy_from_slice(&dev_addr.to_le_bytes());
        ai[10..14].copy_from_slice(&f_cnt.to_le_bytes());
        ai[15] = i;
        let mut block = aes::Block::from(ai);
        cipher.encrypt_block(&mut block); // encrypt_block, not decrypt_block — AES-CTR is symmetric
        keystream.extend_from_slice(&block);
    }

    payload.iter().zip(keystream.iter()).map(|(p, k)| p ^ k).collect()
}
```

---

### FCnt 32-bit Extension

[ASSUMED — derived from D-08 algorithm spec and PITFALLS CP-2 pattern. Not yet compiled.]

```rust
/// LoRaWAN 1.0.x §4.3.1.5 — extend 16-bit OTA FCnt to 32-bit server counter.
///
/// Returns Ok(reconstructed_fcnt) on accept, Err on duplicate/gap exceeded.
pub fn extend_fcnt(wire_u16: u16, session_fcnt: u32) -> Result<u32, FcntError> {
    const MAX_FCNT_GAP: u32 = 16384; // LoRaWAN spec §4.3.1.5

    let candidate_low = (session_fcnt & 0xFFFF_0000) | (wire_u16 as u32);
    let candidate_high = candidate_low.wrapping_add(0x1_0000);

    let reconstructed = if candidate_low > session_fcnt {
        candidate_low
    } else if session_fcnt.wrapping_sub(candidate_low) <= MAX_FCNT_GAP {
        // Within gap window from session counter — likely duplicate or small replay
        return Err(FcntError::Duplicate);
    } else if candidate_high.wrapping_sub(session_fcnt) < MAX_FCNT_GAP {
        // Rollover: candidate_low wrapped below session_fcnt but high candidate is close
        candidate_high
    } else {
        return Err(FcntError::GapExceeded);
    };

    Ok(reconstructed)
}

#[derive(Debug, PartialEq)]
pub enum FcntError {
    Duplicate,
    GapExceeded,
}
```

**Note:** D-08 in CONTEXT.md specifies a slightly simplified algorithm. The pattern above
extends it to also handle the `MAX_FCNT_GAP` check (SE-4 in PITFALLS). The planner should
decide whether to match D-08 exactly (simpler, no gap check) or add the gap check. The spec
requires the gap check; D-08 omits it. Recommend: include the gap check.

---

### SQLite Dedup Query Pattern

[VERIFIED: schema analysis + SQL correctness verified by reading existing codebase patterns]

```sql
-- Add to uplinks table (migration in sql.rs):
ALTER TABLE uplinks ADD COLUMN received_at_ms INTEGER;

-- Composite index for dedup query performance:
CREATE INDEX IF NOT EXISTS idx_uplinks_dedup ON uplinks(dev_addr, f_cnt, received_at_ms);

-- Dedup check query (30-second window = 30000ms):
SELECT COUNT(*) FROM uplinks
WHERE dev_addr = ?1
  AND f_cnt = ?2
  AND received_at_ms > ?3 - 30000;
```

Rust implementation in `repos.rs`:

```rust
// Add to UplinkRepository trait in maverick-core/src/ports/uplink_repository.rs:
async fn is_duplicate(&self, dev_addr: DevAddr, f_cnt: u32, window_ms: i64) -> AppResult<bool>;

// Implement in maverick-adapter-persistence-sqlite/src/persistence/repos.rs:
async fn is_duplicate(&self, dev_addr: DevAddr, f_cnt: u32, window_ms: i64) -> AppResult<bool> {
    let this = self.clone();
    let key = dev_addr.0 as i64;
    let f_cnt_i = f_cnt as i64;
    this.run_blocking(move |p| {
        p.run_with_busy_retry(|conn| {
            let now = now_ms().0;
            let count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM uplinks WHERE dev_addr = ?1 AND f_cnt = ?2 AND received_at_ms > ?3",
                rusqlite::params![key, f_cnt_i, now - window_ms],
                |row| row.get(0),
            )?;
            Ok(count > 0)
        })
    })
    .await
}
```

---

### Mutex `.expect()` → `?` Pattern

[VERIFIED: reading existing codebase; `run_with_busy_retry` already returns `AppResult<T>` where inner returns `rusqlite::Error`]

The existing pattern in `busy.rs` maps `rusqlite::Error` to `AppError::Infrastructure`.
The `.expect()` calls in `lns_ops.rs` use `parse_hex_*` functions. These need to return
`Result<_, rusqlite::Error>` (or a custom error type that converts).

The cleanest approach: make `parse_hex_*` return `Result<T, rusqlite::Error>` using
`rusqlite::Error::InvalidColumnType` or a string error:

```rust
fn parse_hex_dev_eui_result(s: &str) -> Result<[u8; 8], rusqlite::Error> {
    parse_hex_dev_eui(s).map_err(|e| rusqlite::Error::InvalidParameterName(
        format!("invalid dev_eui hex: {e}")
    ))
}

// Then in apply_lns_config_inner (inside transaction, returns rusqlite::Error):
let dev_eui_b = parse_hex_dev_eui_result(&d.dev_eui)?;  // replaces .expect()
```

`apply_lns_config_inner` already returns `Result<(), rusqlite::Error>`, so `?` composes
directly. No new error types needed.

---

### process::exit → Result Propagation Pattern

[VERIFIED: reading main.rs which uses `match cli.command { ... }` dispatch pattern]

```rust
// In commands/config.rs — convert from:
pub(crate) fn run_config_init(config_path: PathBuf, force: bool) {
    if config_path.exists() && !force {
        eprintln!("config file already exists...");
        std::process::exit(2);
    }
    // ...
}

// To:
pub(crate) fn run_config_init(config_path: PathBuf, force: bool) -> Result<(), (i32, String)> {
    if config_path.exists() && !force {
        return Err((2, "config file already exists...".to_string()));
    }
    // ...
    Ok(())
}

// In main.rs — add WAL checkpoint before exit:
fn run_and_exit(result: Result<(), (i32, String)>) -> ! {
    match result {
        Ok(()) => std::process::exit(0),
        Err((code, msg)) => {
            eprintln!("{msg}");
            std::process::exit(code);
        }
    }
}
```

**WAL checkpoint (D-19):** Add explicit `close()` to `SqlitePersistence`:

```rust
impl SqlitePersistence {
    /// Checkpoint the WAL before process exit. Call from main() before std::process::exit.
    pub fn close(self) -> AppResult<()> {
        let guard = self.inner.conn.lock()
            .map_err(|_| AppError::Infrastructure("mutex_poisoned".to_string()))?;
        guard.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")?;
        Ok(())
    }
}
```

Note: `Drop` on `Connection` does NOT run WAL checkpoint automatically in rusqlite 0.33.
Explicit `PRAGMA wal_checkpoint(TRUNCATE)` is required.

---

### Region Inference Fix

[VERIFIED: CP-4 from PITFALLS.md — exact frequency ranges from LoRaWAN Regional Parameters RP002]

```rust
// Current broken code in gwmp.rs::infer_region():
// The match arm order causes US915 (902–928 MHz) to shadow AU915 (915–928 MHz)

// Fixed: use specific channel-plan frequencies instead of overlapping ranges.
// LoRaWAN Regional Parameters RP002-1.0.4:
// US915 uplink: 902.3–914.9 MHz (channels 0-63) + 903.0–914.2 MHz (500kHz channels 64-71)
// AU915 uplink: 915.2–927.8 MHz (125kHz) + 915.9–927.1 MHz (500kHz)
// AS923 uplink: 923.2 and 923.4 MHz (primary)

fn infer_region(freq_mhz: f64) -> Option<RegionId> {
    match freq_mhz {
        f if (923.0..=923.5).contains(&f) => Some(RegionId::As923), // AS923 first (most specific)
        f if (915.0..=928.0).contains(&f) => Some(RegionId::Au915), // AU915 before US915
        f if (902.0..=915.0).contains(&f) => Some(RegionId::Us915),
        f if (863.0..=870.0).contains(&f) => Some(RegionId::Eu868),
        f if (433.0..=434.8).contains(&f) => Some(RegionId::Eu433),
        _ => None,
    }
}
```

The key fix: AS923 (923.0–923.5) and AU915 (915.0–928.0) must come BEFORE US915 (902.0–915.0)
because US915 upper boundary (928 MHz) overlaps with AU915.

---

## Runtime State Inventory

> This is a greenfield change with no stored user data. Clean break is approved (CONTEXT.md §Specifics).

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | `sessions` table: existing rows lack `nwk_s_key`/`app_s_key` columns | Schema migration via `ALTER TABLE` (existing pattern in `sql.rs`) |
| Stored data | `uplinks` table: existing rows lack `received_at_ms` column | Schema migration via `ALTER TABLE` |
| Live service config | No running services to update | None |
| OS-registered state | None | None |
| Secrets/env vars | `DEFAULT_GWMP_BIND_ADDR` constant in `cli_constants.rs` | Code change (no env var risk) |
| Build artifacts | None | None |

**Migration notes:**
- `ALTER TABLE sessions ADD COLUMN nwk_s_key BLOB` — existing rows get NULL; acceptable per D-03 (no production users).
- `ALTER TABLE sessions ADD COLUMN app_s_key BLOB` — same.
- `ALTER TABLE uplinks ADD COLUMN received_at_ms INTEGER` — existing rows get NULL; dedup window query handles NULL gracefully via `NULL > (now - 30000)` → false.

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|-------------|-----------|---------|----------|
| `aes 0.9` crate | MIC + payload decrypt | ✓ (crates.io) | 0.9.0 | None needed |
| `cmac 0.8` crate | MIC CMAC | ✓ (crates.io) | 0.8.0 | None needed |
| Rust stable toolchain | All builds | ✓ | CI-resolved | None needed |
| SQLite (bundled) | rusqlite 0.33 bundled | ✓ | 3.46.x (bundled) | None needed |

**No missing dependencies.** All required crates are available on crates.io. The `aes` and
`cmac` crates are pure Rust with no system dependencies and cross-compile cleanly to armv7/aarch64.

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in `#[test]` / `#[tokio::test]` |
| Config file | none (workspace-level `cargo test`) |
| Quick run command | `cargo test -p maverick-core -- --nocapture 2>&1` |
| Full suite command | `cargo test --workspace 2>&1` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| PROT-01 | MIC reject on wrong MIC bytes | unit | `cargo test -p maverick-core mic_invalid` | ❌ Wave 0 |
| PROT-01 | MIC accept on correct MIC bytes | unit | `cargo test -p maverick-core mic_valid` | ❌ Wave 0 |
| PROT-02 | FCnt 32-bit rollover accept (0xFFFE→0xFFFF→0x0001) | unit | `cargo test -p maverick-core fcnt_rollover` | ❌ Wave 0 |
| PROT-02 | FCnt duplicate reject | unit | already exists `rejects_duplicate_fcnt` | ✅ |
| PROT-03 | SessionSnapshot carries nwk_s_key / app_s_key | unit | `cargo test -p maverick-domain` | ❌ Wave 0 |
| PROT-04 | Payload decrypted and stored | integration | `cargo test -p maverick-integration-tests` | partial |
| PROT-05 | AU915 freq inferred correctly (not US915) | unit | `cargo test -p maverick-adapter-radio-udp region_au915` | ❌ Wave 0 |
| PROT-06 | Duplicate frame discarded silently | integration | `cargo test -p maverick-integration-tests dedup` | ❌ Wave 0 |
| RELI-01 | No panics on invalid hex in lns_ops | unit | `cargo test -p maverick-adapter-persistence-sqlite parse_hex` | ❌ Wave 0 |
| RELI-02 | WAL checkpoint called before exit | manual | verify via `PRAGMA wal_dbsize` | manual-only |
| SEC-01 | DEFAULT_GWMP_BIND_ADDR is 127.0.0.1 | unit | `cargo test -p maverick-runtime-edge bind_default` | ❌ Wave 0 |
| CORE-01 | No external HTTP deps in workspace | static | `cargo tree -p maverick-runtime-edge | grep -E "reqwest|hyper|h2"` | — |
| CORE-02 | Uplink persisted before audit success | integration | existing `ingest_happy_path_updates_session_and_uplink` | ✅ |

### Wave 0 Gaps

- [ ] `crates/maverick-core/src/protocol/lorawan_10x_class_a.rs` — add FCnt rollover + MIC tests
- [ ] `crates/maverick-core/src/use_cases/` — add MIC reject test (mock NwkSKey mismatch)
- [ ] `crates/maverick-adapter-radio-udp/src/tests/` — add region inference AU915/AS923 tests
- [ ] `crates/maverick-adapter-persistence-sqlite/src/persistence/tests/` — add dedup + parse_hex_result tests
- [ ] `crates/maverick-runtime-edge/src/tests/` — add bind_default constant test

---

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | yes (MIC = device auth) | AES-128 CMAC per LoRaWAN 1.0.x §4.4 |
| V3 Session Management | yes (FCnt = replay prevention) | 32-bit FCnt + MAX_FCNT_GAP |
| V4 Access Control | partial (bind address) | Default `127.0.0.1`; operator must explicitly widen |
| V5 Input Validation | yes | hex parsing returns Result; region inference has explicit fallback |
| V6 Cryptography | yes | RustCrypto (constant-time); never hand-rolled |

### Known Threat Patterns for this Stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Frame injection (forged uplink) | Spoofing | MIC verification (PROT-01) |
| Frame replay | Repudiation | FCnt monotonic + dedup window (PROT-02, PROT-06) |
| Key material exposure in logs | Information Disclosure | Keys stored as `[u8;16]`, never formatted into strings |
| GWMP port exposed to internet | Elevation of Privilege | Default bind `127.0.0.1` (SEC-01) |
| SQLite Mutex permanent brick | Denial of Service | `.expect()` removal (RELI-01) |

**SEC-02 (key encryption at rest) is deferred to Phase 4** per REQUIREMENTS.md traceability.
Phase 1 stores keys as plaintext BLOB — document in schema comment.

---

## Open Questions

1. **Should `extend_fcnt` include `MAX_FCNT_GAP` check (SE-4) or match D-08 exactly?**
   - D-08 in CONTEXT.md omits the gap check. SE-4 in PITFALLS.md says gap check is required per spec.
   - Recommendation: include gap check; return a new `RejectFrameCounterGapExceeded` variant to improve observability over `RejectDuplicateFrameCounter`.

2. **`UplinkObservation.f_cnt`: change to `u16` (D-09) or add `wire_f_cnt: u16` alongside `u32`?**
   - Changing to `u16` is cleaner (D-09) but breaks existing tests at `obs(fc: u32)` in `ingest_uplink.rs`.
   - Adding `wire_f_cnt: u16` preserves `f_cnt: u32` for the reconstructed value but adds confusion.
   - Recommendation: change to `u16` as D-09 specifies; update tests to pass `u16` literals.

3. **Dedup window: is 30 seconds appropriate for LoRaWAN?**
   - LoRaWAN gateways emit duplicate `rxpk` entries within ~5 seconds of each other (two gateways hearing the same uplink).
   - 30 seconds is conservative. The spec does not mandate a value; operator concern is multi-gateway deployments where the second copy arrives up to ~10 seconds later due to routing.
   - Recommendation: 30 seconds is correct for the default. Document and make configurable as D-12 specifies.

4. **Raw phy bytes in `UplinkObservation`?**
   - MIC verification in `IngestUplink::execute` requires the 4 MIC bytes stripped by the parser.
   - Either parser must expose `wire_mic: [u8; 4]` field, or use-case must receive full raw phy and do its own strip.
   - Recommendation: add `wire_mic: [u8; 4]` and `phy_without_mic: Vec<u8>` to `UplinkObservation` (or the parser returns a separate `RawUplinkFrame` struct alongside `UplinkObservation`).

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `DROP` on `Connection` does not trigger WAL checkpoint in rusqlite 0.33 | Code Examples — WAL close | If wrong, explicit `close()` is unnecessary but harmless |
| A2 | `extend_fcnt` rollover algorithm in D-08 handles the `wrapping_sub` underflow safely | Code Examples — FCnt | If wrong, devices near 16-bit boundaries brick after rollover |
| A3 | Dedup `NULL > (now - 30000)` evaluates to false in SQLite | Runtime State Inventory | If wrong, all existing uplinks (NULL received_at_ms) would be considered non-duplicate — which is the correct behavior for existing rows, so risk is acceptable |
| A4 | AU915/AS923 region fix with frequency boundary (915.0 MHz) is correct per LoRaWAN RP002-1.0.4 | Code Examples — region fix | If wrong, some AU915 edge-case frequencies might still conflict; exact channel plan tables are the authoritative source |

---

## Sources

### Primary (HIGH confidence)

- Crates.io registry: `cargo search aes` → `aes = "0.9.0"`, `cargo search cmac` → `cmac = "0.8.0"` [VERIFIED 2026-04-16]
- Compilation test: `aes 0.9.0` + `cmac 0.8.0` compiled and ran successfully [VERIFIED 2026-04-16]
- Codebase audit: `grep -rn "process::exit"` → 35 call sites in `commands.rs` + `commands/config.rs` + `gwmp_loop.rs` [VERIFIED 2026-04-16]
- Codebase read: `lns_ops.rs` lines 288, 295-296, 312-313, 317, 327, 332, 382, 399-400 — all `.expect()` inside Mutex lock scope [VERIFIED 2026-04-16]
- Codebase read: `schema.sql` — `uplinks` table has no `received_at_ms` column [VERIFIED 2026-04-16]
- Codebase read: `UplinkObservation.f_cnt` is currently `u32` in `radio_transport.rs` [VERIFIED 2026-04-16]
- Codebase read: `Cargo.toml` `[profile.release]` has `panic = "abort"` (line 58) [VERIFIED 2026-04-16]

### Secondary (MEDIUM confidence)

- STACK.md LoRaWAN 1.0.x B0 block layout — from project's own prior research
- PITFALLS.md CP-2 FCnt rollover algorithm, SE-4 MAX_FCNT_GAP — from project's own prior research

### Tertiary (LOW confidence / ASSUMED)

- LoRaWAN RP002-1.0.4 exact frequency boundaries for AU915/AS923/US915 channel plans — from training knowledge; not verified against the spec document in this session
- `MAX_FCNT_GAP = 16384` per LoRaWAN spec §4.3.1.5 — from PITFALLS.md (itself from training data)

---

## Metadata

**Confidence breakdown:**
- Crypto API (aes + cmac versions and usage): HIGH — compiled and ran 2026-04-16
- B0 block byte layout: HIGH — from STACK.md + PITFALLS.md (project's own prior research) + verified struct by compilation
- FCnt 32-bit algorithm: MEDIUM — algorithm from CONTEXT.md D-08 and PITFALLS.md CP-2; exact edge-case handling (wrapping_sub) is ASSUMED
- SQLite dedup pattern: HIGH — follows existing codebase patterns exactly; schema change verified by reading schema.sql
- process::exit scope (35 sites): HIGH — verified by grep
- .expect() locations: HIGH — verified by reading lns_ops.rs
- Region frequency boundaries: MEDIUM — from training data, not verified against RP002 spec document

**Research date:** 2026-04-16
**Valid until:** 2026-07-16 (RustCrypto crates are stable; LoRaWAN spec is frozen)
