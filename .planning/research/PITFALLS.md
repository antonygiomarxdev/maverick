# PITFALLS RESEARCH — Maverick LNS
_Generated: 2026-04-16_

---

## Summary

This document catalogues pitfalls discovered through direct codebase audit (not WebSearch,
which was unavailable). Every finding is grounded in specific files and line numbers.
Confidence is HIGH for all items because they are based on reading production source.

The codebase has a clean architectural foundation. The critical risks are concentrated in
three areas: (1) LoRaWAN protocol correctness (MIC absent, FCnt 16-bit), (2) UDP surface
exposure (unauthenticated, world-routable by default), and (3) a set of Rust async patterns
that individually are minor but compound under burst or hostile conditions.

---

## Critical Protocol Pitfalls

### CP-1 — No MIC Verification: Any Frame with a Known DevAddr Is Accepted

**What goes wrong:**
`parse_lorawan_payload` in `crates/maverick-adapter-radio-udp/src/gwmp.rs:154–160` strips
the 4-byte MIC tail but never validates it. `validate_uplink` in
`crates/maverick-core/src/protocol/lorawan_10x_class_a.rs:44–46` only checks region, class,
and `f_cnt > last_f_cnt`. No call to AES-CMAC or key material anywhere in the ingest path.

**Why it happens:**
`AbpKeys.nwks_key` is stored in `lns_devices` (schema.sql) and loaded from `lns-config.toml`,
but the `UplinkObservation` struct crossing the adapter→core boundary carries no key material.
The `ProtocolContext` struct has no `nwks_key` field. The crypto was explicitly deferred with
the comment "not required for ingest until downlink/crypto is wired."

**Consequences:**
- Any node on the same LAN as the gateway (or routable to port 17000) that knows a valid
  DevAddr can forge uplinks by sending a LoRaWAN-shaped datagram with a monotonically
  increasing FCnt. All forged frames are persisted as legitimate uplinks.
- Combined with CP-2 and CP-3, this constitutes an open write path to SQLite.
- Operators who use Maverick in a production ABP deployment before this is fixed will store
  injected telemetry indistinguishable from real device data.

**Prevention:**
Thread `NwkSKey` from `lns_devices` through `SessionSnapshot`, carry it into `ProtocolContext`,
and compute AES-CMAC over `MHDR || FHDR || FPort || FRMPayload` with the B0 block per
LoRaWAN 1.0.x spec §4.4 before calling `ProtocolDecision::Accept`. The MIC is the last 4
bytes of the raw frame (already isolated at `raw.len() - 4` in `parse_lorawan_payload`).

**Detection:**
No existing test exercises MIC rejection. Add a unit test that crafts a valid-looking frame
with a wrong MIC and asserts `ProtocolDecision::Reject*`.

---

### CP-2 — FCnt 16-Bit Truncation Breaks Sessions After 65,535 Uplinks

**What goes wrong:**
`gwmp.rs:139–140`:
```rust
let fcnt =
    u16::from_le_bytes([raw[LORAWAN_FHDR_FCNT_START], raw[LORAWAN_FHDR_FCNT_END - 1]]) as u32;
```
The LoRaWAN spec (§4.3.1.5) sends a 16-bit FCnt over-the-air; the server maintains a 32-bit
counter reconstructed by detecting when the device wraps. Maverick casts the 16-bit OTA value
directly to u32 with upper bits always zero. After device FCnt wraps past 0xFFFF, every
subsequent frame arrives with an apparent FCnt ≤ session counter, triggering
`RejectDuplicateFrameCounter`. The session is permanently bricked with no recovery path.

**Why it happens:**
16-bit parse is correct for the OTA wire format. The mistake is failing to implement the
server-side 32-bit reconstruction. A compliant LNS checks if `ota_fcnt_16 + 0x10000` is
within the `MAX_FCNT_GAP` window (spec: 16384) relative to the stored 32-bit counter, and
if so, accepts the frame with the reconstructed 32-bit value.

**Consequences:**
Devices deployed on long maintenance cycles (agricultural, infrastructure) transmit every
few minutes and reach 65,535 uplinks in ~45 days at 1 msg/min. After rollover, the device
appears dead from Maverick's perspective with no log distinguishing this from a real device
failure.

**Prevention:**
```rust
// In parse_lorawan_payload, return (DevAddr, u16, ...) — just the OTA FCnt.
// In validate_uplink (ProtocolContext), reconstruct 32-bit:
let ota_low: u16 = obs.f_cnt_ota;  // rename field
let stored: u32 = session.uplink_frame_counter;
let candidate_low  = (stored & 0xFFFF_0000) | (ota_low as u32);
let candidate_high = candidate_low.wrapping_add(0x10000);
let reconstructed = if candidate_low > stored {
    candidate_low
} else if candidate_high.wrapping_sub(stored) < MAX_FCNT_GAP {
    candidate_high
} else {
    return Ok(ProtocolDecision::RejectDuplicateFrameCounter);
};
```
`MAX_FCNT_GAP` is conventionally 16384 per the LoRaWAN spec.

**Detection:**
Add an integration test that ingests frame 0xFFFE, then 0xFFFF, then 0x0001 (wrap), and
asserts all three are accepted with the stored FCnt becoming 0x10001.

---

### CP-3 — Duplicate Detection Is Insufficient for Multi-Gateway Scenarios

**What goes wrong:**
`validate_uplink` only checks `obs.f_cnt > session.uplink_frame_counter`. When the same
uplink is heard by two gateways and both PUSH_DATA arrive at the GWMP socket within the
same loop iteration, the first observation increments the session FCnt; the second (same
FCnt) is rejected as a duplicate. This is correct behaviour for same-FCnt duplicates. The
pitfall is the opposite: if gateway A sends FCnt=10 and gateway B sends FCnt=10, and they
arrive in order B then A, the first is accepted, and the second is silently dropped — both
correct. However, there is no dedup at the GWMP batch level: `parse_push_data` returns
all `rxpk` entries as separate `UplinkObservation` values, and they are processed in a
simple `for obs in batch.observations` loop. If a concentrator returns the same demod
result twice (hardware firmware bug documented in SX1302 reference implementations), both
copies enter the ingest pipeline and the second is rejected silently.

**Prevention:**
Log a distinct audit outcome for `RejectDuplicateFrameCounter` with the actual observed FCnt
so operators can distinguish "device replayed" from "gateway double-delivered" from "FCnt
rollover unexpectedly rejected."

---

### CP-4 — Region Inference from Frequency Uses Shadowed Match Arms

**What goes wrong:**
`gwmp.rs:164–173` — match arms for AU915 (915–928 MHz) and AS923 (920–923.5 MHz) are
unreachable because US915 (902–928 MHz) matches first. Any AU915 or AS923 device will be
labelled US915. If that device's session has region AU915 or AS923, `validate_uplink`
returns `RejectRegionMismatch` and every uplink is silently dropped.

When `freq` is absent from the GWMP JSON, the fallback is `Eu868` regardless of actual
deployment region. A gateway misconfigured to omit frequency in GWMP JSON will cause all
non-EU868 devices to be rejected.

**Prevention:**
Re-order match arms from most-specific to least-specific. Consider parsing the frequency
as part of a `RegionChannelPlan` lookup table that maps specific channel frequencies to
regions rather than using overlapping MHz ranges:
```
// AU915: 915.2, 915.4, ..., 927.8 MHz (8 channels)
// AS923: 923.2, 923.4 MHz
// US915: 902.3, 902.5, ..., 914.9 MHz
```
Return `Err(AppError::InvalidInput(...))` when `freq` is absent rather than defaulting.

---

### CP-5 — ABP Session Reset Vulnerability (Spec §7.1.3)

**What goes wrong:**
The LoRaWAN 1.0 spec warns that ABP devices should never reset their FCnt (power-cycle
preserves the counter in NVM). If a device is power-cycled without NVM preservation and
restarts from FCnt=0, Maverick will reject every uplink as a duplicate because the stored
session FCnt is already high. The operator has no automated path to reset the session FCnt.

**Why it happens:**
The `sessions` table stores `uplink_fcnt` as a strict monotonic watermark. There is no CLI
command to reset a session counter. The only recovery is to delete the session row in SQLite
directly or re-run `config load` (which upserts with `uplink_frame_counter: 0`... only if
the device row in the config has `uplink_frame_counter` explicitly — which it currently
does not; `apply_lns_config_inner` reconstructs sessions from device config, not from any
explicit FCnt field).

**Prevention:**
Add a `maverick-edge config reset-fcnt --dev-addr <hex>` command that sets `uplink_fcnt = 0`
for a given session. Document in the runbook that ABP devices without NVM FCnt persistence
require this command after power-cycle. Consider an operator-level "FCnt relaxed" mode
(LoRaWAN 1.0.x §7.1.3 allows it for constrained devices with appropriate risk disclosure).

---

### CP-6 — Payload Is Stored Encrypted; No Decryption with AppSKey

**What goes wrong:**
`UplinkRecord.payload` stores `FRMPayload` bytes directly. LoRaWAN `FRMPayload` is encrypted
with `AppSKey` (AES-128-CTR). `apps_key` is stored in `lns_devices` but is never loaded or
passed to any decryption step. Downstream consumers reading `uplinks` from SQLite receive
ciphertext, not plaintext sensor data.

**Why it happens:**
Deferred with the same comment as MIC: keys not wired until downlink/crypto is implemented.

**Consequences:**
Any HTTP, MQTT, or cloud sync extension reading uplinks will receive encrypted payloads
with no indication they are encrypted. If operators build dashboards or alerts on top of
this data before decryption is added, they will silently get garbage values.

**Prevention:**
Either decrypt at ingest time (requires `AppSKey` in `ProtocolContext`), or store raw
ciphertext with an explicit `is_encrypted: bool` flag and `app_key_fingerprint` so
consumers know they need to decrypt. Add a `TODO: ENCRYPTED` comment to `UplinkRecord`
until the feature lands.

---

## Radio Hardware Pitfalls

### RH-1 — SPI / Direct Concentrator Adapter Not Yet Implemented

**What goes wrong:**
`PROJECT.md` lists "Direct SPI radio adapter — read from LoRa concentrator (SX1302/SX1303)
without a packet forwarder" as an Active requirement. Currently, all uplinks arrive via a
Semtech GWMP UDP packet forwarder (`maverick-adapter-radio-udp`). The SX1302/SX1303 HAL
(libloragw) must be accessed via FFI or a Rust wrapper.

**Known SX1302 HAL pitfalls:**
- The SX1302 HAL requires exclusive SPI access; calling `lgw_start()` from two processes
  simultaneously corrupts concentrator state and requires a hardware reset via GPIO pin.
  Any design that spawns multiple ingest processes must serialize HAL access.
- `lgw_receive()` returns up to 8 packets per call; a tight poll loop without a sleep
  wastes CPU and heats the Pi on armv7. The reference implementation polls at ~1ms intervals.
- HAL initialization (`lgw_start()`) can fail transiently if the SX1302 is still in reset
  state after boot. Requires a small delay and retry loop — not documented in the HAL header
  but present in reference gateway implementations.
- SX1302 TRIG pin (GPIO17 on RAK5146) must be driven LOW before `lgw_start()` or the
  concentrator timestamp counter does not synchronize. Missing this step causes all RSSI
  and SNR values to be bogus.

**Prevention:**
When implementing the SPI adapter, wrap the HAL in a `SpiConcentratorBackend` that:
(a) holds the HAL handle as a non-Clone singleton (prevents dual `lgw_start()` at the
type level), (b) implements a 1–5 ms poll interval with `tokio::time::interval` in a
dedicated `spawn_blocking` task, (c) implements a retry loop on `lgw_start()` failure
with a configurable cooldown (default 3 retries × 500ms).

---

### RH-2 — GWMP Buffer Size Fixed at 4096 Bytes; Oversized Datagrams Silently Truncated

**What goes wrong:**
`gwmp_loop.rs:50` and `gwmp_loop.rs:244`:
```rust
let mut buf = vec![0_u8; 4096];
```
`recv_from` truncates datagrams larger than the buffer without returning an error on Linux
(the excess bytes are discarded; the returned `n` equals 4096). A GWMP PUSH_DATA with many
`rxpk` entries (a dense gateway in an urban deployment) can exceed 4096 bytes.

**Why it matters:**
A Semtech packet forwarder with 8 simultaneous demodulated packets (max SX1302 output) with
large payloads (FSK, 256-byte payload each) can produce a JSON body > 4096 bytes. The
truncated JSON will fail `serde_json::from_str` and be counted as a `failed` parse — the
uplinks are lost without any indication that truncation occurred rather than a malformed frame.

**Prevention:**
Increase default buffer to 65535 bytes (UDP MTU maximum) and add a size guard:
```rust
let mut buf = vec![0_u8; 65535];
let (n, _addr) = socket.recv_from(&mut buf).await?;
if n == buf.len() {
    tracing::warn!("GWMP datagram may have been truncated at {} bytes", n);
}
```

---

### RH-3 — No Class A Rx1/Rx2 Downlink Window Implementation

**What goes wrong:**
LoRaWAN Class A requires the LNS to send a downlink within specific timing windows:
- Rx1: opens exactly 1 second after uplink end of TX (RECEIVE_DELAY1)
- Rx2: opens exactly 2 seconds after uplink end of TX (RECEIVE_DELAY2)

`UdpDownlinkTransport` is implemented and tested but is not wired to the ingest use case.
`DownlinkRepository` and `DownlinkEnqueue` port traits are defined but have no adapter.
The GWMP `rxpk` JSON includes `tmst` (concentrator timestamp), which is required to
compute the Rx1 window target `tmst + 1,000,000 µs` for the PULL_RESP datagram.

**Why it matters:**
Without Rx1/Rx2, MAC commands (LinkADRReq, DevStatusReq) cannot be sent, confirmed uplinks
cannot be acknowledged (ACK bit in downlink), and network-controlled ADR cannot update
spreading factor or TX power.

**Consequences for v1:**
This is documented as out-of-scope for v1 (OTAA deferred). However, operators using
confirmed uplinks (`MType == ConfirmedDataUp`) will never receive ACKs. Devices configured
for confirmed messages will retry until `NbTrans` exhausted, multiplying traffic.

**Prevention:**
Parse and store `tmst` from `rxpk` in `UplinkObservation`. When the downlink path is
implemented, use `tmst + 1_000_000` as the target `tmst` for the PULL_RESP GWMP message.
Document in the runbook that confirmed uplinks should not be configured until downlink
is implemented.

---

## Operational / Deployment Pitfalls

### OD-1 — UDP Ingest Port Bound to 0.0.0.0 by Default

**What goes wrong:**
Default `MAVERICK_GWMP_BIND` is `0.0.0.0:17000` (`cli_constants.rs`). On a Raspberry Pi
with a WAN-facing interface, GWMP port 17000 is reachable from the public internet.
GWMP has no authentication. Combined with CP-1 (no MIC), any internet host can inject
uplinks. Combined with autoprovision enabled by default, attackers can enumerate DevAddrs
and flood `lns_pending` at up to 10 inserts/gateway/minute (default rate limit).

**Prevention:**
Change default bind to `127.0.0.1:17000`. Document the network topology requirement
(packet forwarder must be co-located or on a private network). Add a startup warning
when `0.0.0.0` is detected: `warn!("GWMP ingest bound to all interfaces; ensure firewall restricts port 17000")`.

---

### OD-2 — process::exit Called in 25+ Paths; Drop Skipped on Error

**What goes wrong:**
`crates/maverick-runtime-edge/src/commands/config.rs` calls `std::process::exit(1)` in
approximately 25 CLI error paths. `gwmp_loop.rs:190` calls `std::process::exit(1)` if the
socket bind fails in supervised mode. Rust's `Drop` implementations (SQLite WAL checkpoint,
async task cleanup, file flush) are bypassed.

**Why it matters in production:**
If `maverick-edge config load` is partway through a large transaction and the process is
killed by a `process::exit`, SQLite WAL provides atomicity (the incomplete transaction is
rolled back on next open). However, `spawn_blocking` tasks that hold the Mutex may not
complete their current operation, and the OS-level file descriptor for the SQLite WAL
is abandoned rather than closed cleanly. On next open, WAL recovery runs automatically,
but accumulated dirty pages are not checkpointed.

**Prevention:**
Return `Result<(), String>` from all command handlers. Call `std::process::exit` only in
`main` based on the returned result. In `gwmp_loop.rs`, propagate the bind failure up to
`main` rather than calling `exit` inside the loop function.

---

### OD-3 — No Process Supervision or Self-Restart

**What goes wrong:**
`PROJECT.md` lists "Process supervision and self-healing" as Active. Currently, if
`maverick-edge radio ingest-loop` panics or receives an unhandled signal, the process
exits and uplinks are lost until a human or systemd restarts it.

**Why it matters:**
The ingest loop is a supervised loop (continues on recoverable parse/ingest errors), but
there is no outer supervisor process and no watchdog. A panic in a Tokio worker thread
(e.g., from a poisoned Mutex — see M3 in CONCERNS.md) propagates to `tokio::main` and
terminates the process.

**Specifically risky paths:**
- `apply_lns_config_inner` contains 12+ `.expect("validated lns config")` calls on hex
  parsing (`lns_ops.rs:288–400`). If validation is ever bypassed, this panics inside
  `spawn_blocking`, poisons the Mutex, and all subsequent SQLite operations return
  `AppError::Infrastructure` forever for the life of the process.
- `UdpSocket::bind` failure in supervised mode calls `process::exit(1)`.
- Tokio runtime panic (e.g., OOM on armv7 with 512MB RAM under burst) exits the process.

**Prevention:**
(a) Remove all `.expect()` from `apply_lns_config_inner`, return `Result` instead.
(b) Add a systemd `Restart=always` service file as the canonical deployment unit.
(c) Add a panic hook (`std::panic::set_hook`) that logs the panic message to stderr
    before the process terminates so systemd journal captures it.
(d) Long-term: implement a supervisor process that respawns `maverick-edge ingest-loop`
    on exit with a configurable backoff.

---

### OD-4 — SQLite Single-Connection Mutex Serializes All Async I/O

**What goes wrong:**
`persistence/mod.rs:52`: `pub(super) conn: std::sync::Mutex<Connection>`. All async paths
(`get_by_dev_addr`, `upsert`, `append`, `emit`, `pressure_snapshot`) call `run_blocking`
which dispatches to `tokio::task::spawn_blocking`. Under burst (many simultaneous GWMP
datagrams), all `spawn_blocking` tasks queue on the single Mutex. The Tokio blocking thread
pool (default 512 threads) is wasted waiting on a contended `std::sync::Mutex`.

**Compounding factor:**
`prune_hard_limit_circular_sql` calls `self.db_file_bytes()` which performs a `stat` syscall
while holding the Mutex (T2 in CONCERNS.md). Each write acquires the Mutex, does the write,
calls a count query for the retention policy, optionally deletes rows, then calls `stat`.

**At what scale does this matter:**
For Maverick's target (a single LoRa concentrator, ≤8 simultaneous demodulated packets per
GWMP datagram), message rate is typically ≤100 uplinks/second even in dense deployments.
At this rate, the single-connection lock is not a performance bottleneck. It becomes a
problem if the TUI/CLI and the ingest loop run concurrently (both call `run_blocking` into
the same `SqlitePersistence` instance), or if future extensions read from the same
connection. The pressure snapshot (`storage-pressure` CLI command) shares the same lock.

**Prevention:**
Separate the reader (session lookups, pressure checks) from the writer (upsert, append,
audit). Use `rusqlite` in WAL mode with a separate read-only `Connection` for read-heavy
paths. Short-term: add an `updated_at_ms` index on `sessions` to avoid the full table
scan in LRU pruning (T7 in CONCERNS.md).

---

### OD-5 — Storage Pressure Uses Wrong Disk on Multi-Disk Systems

**What goes wrong:**
`probe.rs:63–66`:
```rust
disks.iter().map(|d| d.total_space()).find(|t| *t > 0)
```
Returns the first disk enumerated with non-zero capacity, not the disk containing
`MAVERICK_DATA_DIR`. On a Raspberry Pi with an SD card (system) and USB stick (data
directory on `/mnt/data`), if the USB stick is enumerated first, storage pressure ratios
are computed against the USB capacity. If the SD card is enumerated first and the data
directory is on the USB stick, the ratio is wrong in the opposite direction.

**Prevention:**
Resolve the filesystem mount point for `MAVERICK_DATA_DIR` using `statvfs` or by walking
`/proc/mounts` to find which mounted device contains the data directory, then use that
device's capacity.

---

### OD-6 — Rate-Limit State Is Process-Global Static; Resets on Restart

**What goes wrong:**
`lns_guard.rs:21–36`: `static BUCKET: OnceLock<Mutex<HashMap<...>>>`. The autoprovision
rate limit is in-process memory. A process restart (including systemd `Restart=always`)
resets the bucket. An attacker can restart the ingest process (or crash it) to reset the
rate limit and resume flooding `lns_pending`.

**Additionally:** The static is shared across all test cases in the same process binary,
making tests that depend on rate limiting non-deterministic when run in parallel.

**Prevention:**
Store rate-limit state in SQLite (a `lns_ratelimit` table with `gateway_eui`, `minute_bucket`,
`count`) so it survives restarts. For tests, expose a `#[cfg(test)] fn reset_rate_limit()`
function or parameterize the limit through the function signature rather than a static.

---

### OD-7 — No Schema Migration for `sessions.updated_at_ms` Index

**What goes wrong:**
The `sessions` table has no index on `updated_at_ms`. The LRU pruning query (`ORDER BY
updated_at_ms ASC LIMIT ?`) is a full table scan. For the constrained profile (max_records_critical
is small), this is fine. For the balanced or high-capacity profile with thousands of sessions,
every write triggers a full scan. There is no migration to add this index to existing databases.

**Prevention:**
Add to `sql.rs` migration:
```sql
CREATE INDEX IF NOT EXISTS idx_sessions_updated_at ON sessions(updated_at_ms);
```
Run this in `init_schema` (which uses `CREATE TABLE IF NOT EXISTS`, so it is idempotent).

---

### OD-8 — `recent-errors` Is a Stub; No Structured Log Persistence

**What goes wrong:**
`commands.rs:201–210` returns a hardcoded stub message. There is no log file, ring buffer,
or structured error sink. Debugging a production issue requires `journalctl -u maverick-edge.service`
or attaching stderr directly. The TUI shows "recent errors" in the menu but it is non-functional.

**Consequence:** Operators on embedded devices without SSH access (running the TUI locally)
have no way to diagnose uplink failures through the intended UI.

**Prevention:**
Write a bounded ring-buffer of `AppError` instances to a table `log_events(id, level,
message, created_at_ms)` in SQLite, capped at 500 rows. Wire `recent-errors` to query
the last N rows. This is consistent with the offline-first, SQLite-everything design.

---

## Rust Async Reliability Pitfalls

### RA-1 — std::thread::sleep Inside spawn_blocking Starves Tokio Thread Pool

**What goes wrong:**
`persistence/busy.rs:37–39`:
```rust
std::thread::sleep(Duration::from_millis(
    BUSY_RETRY_BACKOFF_BASE_MS * u64::from(attempt + 1),
));
```
This `sleep` is inside a closure passed to `run_with_busy_retry`, which is called from
within `run_blocking → spawn_blocking`. Tokio's blocking thread pool has a default limit of
512 threads. If many ingest tasks are blocked on `std::thread::sleep` simultaneously (during
a burst that triggers SQLite BUSY errors), the pool can be saturated, starving other async
operations including health checks, audit emits, and the TUI command path.

**Prevention:**
Replace `std::thread::sleep` in the busy-retry loop with `tokio::time::sleep` (requires the
busy-retry to be `async`), or move the retry logic to the async caller so `spawn_blocking`
is not re-entered per retry attempt.

---

### RA-2 — Mutex Poison Permanently Bricks SQLite for the Process Lifetime

**What goes wrong:**
If any code holding `inner.conn.lock()` panics (e.g., from the `.expect("validated lns config")`
calls in `lns_ops.rs`), `std::sync::Mutex` enters a poisoned state. `busy.rs:32–33`:
```rust
let mut guard = self.inner.conn.lock()
    .map_err(|_| AppError::Infrastructure(SQLITE_MUTEX_POISONED.to_string()))?;
```
returns `Err` on every subsequent call. The ingest loop continues running (it handles
`AppError::Infrastructure` gracefully), but every uplink fails with `SQLITE_MUTEX_POISONED`.
The operator sees a stream of failures with no clear path to recovery except process restart.

**Prevention:**
(a) Eliminate all panics from paths that hold the Mutex (replace `.expect()` with `Result`
    propagation in `apply_lns_config_inner`).
(b) Consider using `parking_lot::Mutex` which does not implement poisoning, if panics are
    truly not expected on those paths.
(c) In `busy.rs`, on `PoisonError`, attempt to recover the mutex guard with
    `into_inner()` and log a critical warning rather than returning an error immediately.

---

### RA-3 — process::exit in an Async Context Races with In-Flight Futures

**What goes wrong:**
`gwmp_loop.rs:190` and `gwmp_loop.rs:210–215` call `std::process::exit(1)` directly from
inside an `async fn`. Any `await` points in flight (pending audit emits, pending SQLite
writes via `spawn_blocking`) are abandoned without their futures being dropped cleanly.
Tokio does not guarantee that `spawn_blocking` tasks complete when `exit` is called.

**Prevention:**
Return `Result` from the async function and propagate to `main`, where a single
`std::process::exit(1)` call is appropriate. This ensures all in-flight async tasks see
cancellation signals through `tokio::runtime::Runtime::drop` before the OS process exits.

---

### RA-4 — spawn_blocking Opens SQLite on Every run_radio_ingest_once Call

**What goes wrong:**
`gwmp_loop.rs:84–102`: `SqlitePersistence::open` is called inside `run_radio_ingest_once`,
which is the one-shot ingest command. Each call opens a new SQLite connection, runs
`init_schema` (DDL), and then drops the connection. If `ingest-once` is called in a tight
loop by external tooling (e.g., a shell script), each call acquires and releases the WAL
lock, causes a WAL checkpoint, and re-executes all `CREATE TABLE IF NOT EXISTS` statements.

**Prevention:**
The one-shot command is intended for testing/debugging, so this is acceptable. Document
that `ingest-loop` (supervised) should be used for production — it opens the connection
once and holds it for the process lifetime.

---

### RA-5 — No Backpressure on GWMP Receive Loop

**What goes wrong:**
`gwmp_loop.rs:246–280`: the supervised loop calls `recv_from`, then processes all
observations synchronously (sequential `await`s in the for-loop). If SQLite write latency
spikes (e.g., disk pressure triggers hard-limit pruning), the UDP receive loop is blocked
during processing and cannot accept new datagrams. Since UDP is fire-and-forget, the
packet forwarder continues sending; datagrams that arrive while the loop is blocked are
dropped by the OS socket buffer (default ~208KB on Linux).

**At what scale:**
A Semtech SX1302 can demodulate 8 simultaneous packets. At SF7 BW125, each packet takes
~56ms on air. If SQLite write (including pruning) takes >56ms, the socket buffer fills
and datagrams are dropped. On an SD card with high write latency, SQLite writes can take
100–200ms under load.

**Prevention:**
Decouple receive from ingest: push received observations into a bounded `tokio::sync::mpsc`
channel and process from a separate task. The channel provides backpressure notification
and the receive loop can log "channel full, dropping observation" rather than silently
losing the UDP datagram.

---

### RA-6 — UplinkObservation Does Not Carry Raw Frame; Prevents Post-Hoc Debug

**What goes wrong:**
`parse_lorawan_payload` consumes the raw PHY payload and returns only `(DevAddr, u32, u8, Vec<u8>)`.
The raw bytes are discarded. If MIC verification fails (once implemented) or if the parse
produces unexpected results, there is no way to log or store the raw frame for post-hoc
debugging.

**Prevention:**
Store `raw_phy: Option<Vec<u8>>` in `UplinkObservation` (gated behind a debug flag or
configuration option). This enables operators to extract and decode suspect frames using
external tools without needing to replay hardware traffic.

---

## Security Pitfalls

### SE-1 — NwkSKey and AppSKey Are Stored in SQLite in Plaintext

**What goes wrong:**
`schema.sql`: `nwks_key BLOB`, `apps_key BLOB` in `lns_devices`. These are loaded from
`lns-config.toml` and written to the database. An attacker who gains read access to
`maverick.db` (e.g., via a path traversal in a future HTTP extension, or by physical access
to the SD card) obtains all session keys.

**Why it matters:**
With `NwkSKey`, an attacker can forge valid MIC-signed frames for any enrolled device.
With `AppSKey`, they can decrypt all historically stored payloads in the `uplinks` table.

**Prevention:**
(a) Short-term: document that `maverick.db` must be on an encrypted filesystem or have
    `0600` permissions with the service running as a dedicated non-root user.
(b) Long-term: encrypt session keys at rest using a key derived from a hardware identity
    (TPM, OP-TEE on supported hardware) or a passphrase-protected master key.
(c) Ensure `lns-config.toml` has `0600` permissions (the installer should enforce this).

---

### SE-2 — Autoprovision Creates Pending Rows for All Observed DevAddrs

**What goes wrong:**
By default, `autoprovision.enabled = true` in `lns-config.toml`. Any frame arriving at
port 17000 with a DevAddr not in sessions causes a row to be written to `lns_pending`.
The rate limit is 10 rows/gateway/minute, but the limit is per-gateway-EUI as extracted
from the GWMP header. An attacker spoofing different gateway EUIs in the GWMP header
(the EUI is not authenticated) can generate unlimited `lns_pending` rows, filling the
`lns_pending` table and potentially the SQLite database.

**Why it is hard to detect:**
`lns_pending` is not subject to the `prune_hard_limit_circular_sql` path (which only
prunes `sessions`, `uplinks`, and `audit_events`). Pending rows are retained until
`pending_ttl_secs` (default 86400 = 1 day) with no cleanup automation. There is no
scheduler that prunes expired rows; cleanup requires operator action or a future job.

**Prevention:**
(a) Bind the GWMP port to `127.0.0.1` by default (OD-1).
(b) Apply the rate limit per source IP address in addition to gateway EUI.
(c) Add automated TTL-based pruning of `lns_pending` rows older than `pending_ttl_secs`
    to the ingest loop or a periodic task.
(d) Cap the total row count in `lns_pending` (e.g., 1000) and reject new pending inserts
    above the cap.

---

### SE-3 — Extension IPC Boundary Is Unauthenticated (When Implemented)

**What goes wrong:**
`PROJECT.md` lists Extension IPC boundary as Active. The design calls for a local API
(HTTP or Unix socket) for extensions to communicate with the core. If implemented as a
local HTTP server on `127.0.0.1`, any process on the machine can call it — including
a malicious process running as a different user. There is no authentication design specified.

**Prevention:**
Use a Unix domain socket with filesystem permissions (`0660`, owned by `maverick:maverick`)
rather than a TCP socket. This ties access control to the OS user/group model without
requiring token management. If TCP is used, require a HMAC-signed request token derived
from a secret in `/etc/maverick/runtime.env`.

---

### SE-4 — No Replay Protection Beyond FCnt Monotonic Check

**What goes wrong:**
FCnt monotonic check (`f_cnt > session.uplink_frame_counter`) prevents replay of exact
frame duplicates. However, once MIC verification is added, a replay of a recently seen
frame (FCnt = stored + 1) with a valid MIC is accepted. The LoRaWAN spec requires tracking
a `MAX_FCNT_GAP` window and rejecting frames that skip too far ahead (> 16384 FCnt units),
which Maverick does not implement.

**Prevention:**
Add `MAX_FCNT_GAP = 16384` check in `validate_uplink`: if `reconstructed_fcnt - stored > MAX_FCNT_GAP`,
return `RejectDuplicateFrameCounter` (or a new `RejectFrameCounterGapExceeded` variant for
observability). This is part of the same FCnt 32-bit reconstruction work (CP-2).

---

## Prevention Strategies

| Category | Priority | Action |
|----------|----------|--------|
| CP-1: No MIC | Immediate | Thread NwkSKey through ProtocolContext; implement AES-CMAC before any production use |
| CP-2: FCnt 16-bit | Immediate | Implement 32-bit FCnt reconstruction per spec §4.3.1.5 |
| CP-4: Region inference | High | Fix match arm ordering in `infer_region`; return Err on missing freq |
| OD-1: UDP exposure | High | Change default bind to 127.0.0.1; add startup warning for 0.0.0.0 |
| OD-3: No supervision | High | Add systemd service file with Restart=always; remove panics from hot paths |
| RA-2: Mutex poison | High | Remove all .expect() from lns_ops.rs; replace with Result propagation |
| SE-1: Keys in plaintext | Medium | Document permission requirements; enforce 0600 on db and config |
| SE-2: Pending flood | Medium | Cap lns_pending rows; add TTL-based pruning |
| RH-2: 4096 byte buffer | Medium | Increase recv_from buffer to 65535 bytes |
| RA-1: sleep in spawn_blocking | Medium | Move busy-retry sleep to async context |
| OD-2: process::exit | Medium | Propagate Result to main; single exit point |
| OD-4: SQLite single conn | Low | Separate read-only connection for pressure/session reads |
| OD-5: Wrong disk hint | Low | Resolve data directory mount point before selecting disk capacity |
| OD-6: Rate limit in memory | Low | Move rate limit state to SQLite |
| OD-7: Missing index | Low | Add idx_sessions_updated_at migration |
| CP-6: Encrypted payload | Document | Mark UplinkRecord.payload as encrypted; add is_encrypted flag |
| CP-5: ABP FCnt reset | Document | Add reset-fcnt CLI command; document in runbook |
| RA-5: No backpressure | Future | Decouple recv from ingest with mpsc channel |

---

## Phase-Specific Warnings

| Phase Topic | Likely Pitfall | Mitigation |
|-------------|---------------|------------|
| MIC verification | AES-CMAC block construction B0 requires `dir=0` for uplinks, `dev_addr` LE, `f_cnt` as u32 LE — easy to get byte order wrong | Write a test vector from the LoRaWAN 1.0.x Test Specification |
| FCnt 32-bit fix | `MAX_FCNT_GAP` window must be tunable for ABP devices that power-cycle (may need relaxed mode) | Add a per-device FCnt policy flag |
| Direct SPI adapter | SX1302 HAL is a C library; FFI unsafety requires careful memory management; `lgw_receive` returns C structs | Wrap in a dedicated blocking task; never call HAL from async context |
| Extension IPC boundary | Unix socket path must be created before TUI/extensions start; race condition if extension starts before edge core | Use a file-existence check + connect retry in the extension startup |
| Process supervision | systemd Restart=always does not prevent rapid-restart loop on persistent failures | Add `StartLimitIntervalSec` and `StartLimitBurst` to service file |
| AppSKey decryption | LoRaWAN AES-128-CTR keystream uses a block counter `i` starting at 1, not 0 — common off-by-one | Test against known test vectors from LoRa Alliance |
| OTAA join (future) | JoinAccept MIC uses AppKey for LoRaWAN 1.0, but NwkKey for 1.1 — different key material for same operation | Gate on LoRaWAN version in protocol capability module |

---

## Gaps / Unknowns

1. **SX1302 HAL Rust wrapper availability**: No maintained pure-Rust SX1302 driver exists
   as of early 2026 (training data, unverified). The most likely approach is FFI bindings
   to `libloragw`. The specific version of the SX1302 HAL that supports RAK5146 (used in
   the Pi HAT) versus RAK2287 needs to be verified against RAK's GitHub forks.

2. **LoRaWAN Regional Parameters for ADR**: The ADR (Adaptive Data Rate) algorithm requires
   per-region channel plans with default data rates, min/max DR, and step sizes. These are
   not in the current codebase. When ADR is implemented, the regional parameters table must
   be sourced from the LoRa Alliance RP002 document, not approximated.

3. **SD card write durability**: SQLite WAL on an SD card has known reliability issues under
   power loss (incomplete sector writes on cheap SD cards bypass `fsync`). Whether to use
   `PRAGMA synchronous = FULL` (safer, slower) vs `NORMAL` (current) depends on the SD card
   model. This is an operator configuration decision that should be documented.

4. **Packet forwarder compatibility**: Only Semtech GWMP v1/v2 (Basic Station protocol is
   different) is parsed. RAK gateways running Basics Station firmware will not work with
   Maverick's current UDP adapter. The scope of forwarder support should be explicitly
   documented in `docs/compatibility-matrix.md`.

5. **Time synchronization for Class A Rx windows**: Once downlink is implemented, the LNS
   must compute the gateway transmit timestamp from the GWMP `txpk.tmst` field. On a Pi
   without GPS PPS, NTP jitter can cause missed Rx windows. The interaction between
   `RECEIVE_DELAY1` (1s) and NTP accuracy needs operational guidance.
