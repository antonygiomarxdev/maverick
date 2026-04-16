# STACK RESEARCH — Maverick LNS
_Generated: 2026-04-16_

---

## Summary

Maverick's four active stack gaps — SPI radio ingress, LoRaWAN MIC verification, process
supervision, and extension IPC — each have a clear Rust-ecosystem answer. The crypto gap
(AES-128 CMAC) is unambiguously solved by the RustCrypto project with no-std/embedded
compatible crates that are already used widely in the LoRaWAN Rust ecosystem. The SPI radio
gap has no pure-Rust SX1302/SX1303 HAL; the only production path is FFI-binding Semtech's
`libloragw` C library. Process supervision is a Linux-operational concern, not a code
concern: systemd unit + `Restart=always` is the right answer, and no Rust crate is needed
for it. Extension IPC should use Unix domain sockets with a line-delimited JSON framing,
matching what the existing TUI already does implicitly (subprocess model), and matching how
Semtech's own `chirpstack` ecosystem works.

---

## LoRa Concentrator SPI Driver

### Problem
The `maverick-adapter-radio-udp` crate requires a running Semtech packet forwarder process
(`lora_pkt_fwd`) to feed UDP datagrams. The v1 Active requirement is a direct SPI adapter
that reads from the concentrator without an intermediary process.

### The SX1302/SX1303 Reality

There is **no maintained pure-Rust HAL or driver for the SX1302/SX1303 chipset** as of
early 2026. Semtech's own reference implementation is the C library `libloragw` (part of the
`sx1302_hal` repo on GitHub: `Lora-net/sx1302_hal`). This is what every Linux-based LoRa
concentrator — including RAK Wireless HATs — uses under the hood.

The production approach used by all known Rust LNS projects (including ChirpStack's gateway
bridge, BasicStation, and community projects) is:

**Option A — FFI wrapper around libloragw (recommended for v1)**
- Bind `libloragw` via `bindgen` at build time.
- Wrap in a new adapter crate `maverick-adapter-radio-spi`.
- The crate implements the same `UplinkIngressBackend` port trait already defined in
  `maverick-core::ports::uplink_ingress`.
- This is the same pattern RAK's own `rak-lora-concentrator` Rust wrapper uses.

**Relevant crates / references:**
| Crate / Repo | What it is | Status |
|---|---|---|
| `Lora-net/sx1302_hal` (C, GitHub) | Semtech's official HAL; the definitive SPI driver | Actively maintained |
| `bindgen` (crates.io) | Build-time C header → Rust FFI bindings | Stable, widely used |
| `rppal` (crates.io) | Pure Rust SPI/GPIO on Raspberry Pi (Linux spidev) | Stable, actively maintained; useful for GPIO reset/power control of the concentrator HAT but NOT for HAL-level packet receive |

**Option B — Keep GWMP UDP, supervise `lora_pkt_fwd` as a sibling process**
- The packet forwarder process runs alongside `maverick-edge`, supervised by systemd.
- Simpler: no FFI, no build-time C dependency.
- Acceptable for v1 if the direct SPI requirement is about reliability (no extra hop) rather
  than eliminating the packet forwarder binary.
- Downside: requires distributing `lora_pkt_fwd` binary alongside Maverick.

### Recommendation
For v1, **Option B** (supervised sibling process) is the lower-risk path given the
armv7/aarch64 cross-compilation constraint and the absence of a tested pure-Rust SX1302
driver. Option A (FFI) is the correct long-term target but adds non-trivial cross-compilation
complexity (`libloragw` must be cross-compiled for armv7/aarch64 at CI time). Flag this for
deeper research in the SPI adapter phase.

---

## LoRaWAN Crypto (MIC / AES-128 CMAC)

### Problem
MIC verification is completely absent. `maverick-core::protocol::LoRaWAN10xClassA::validate_uplink`
does not check the Message Integrity Code. Any frame with a valid `DevAddr` and incrementing
`FCnt` is accepted.

### LoRaWAN 1.0.x MIC Construction
Per LoRaWAN 1.0.x specification, the MIC is the first 4 bytes of AES-128 CMAC computed over
a B0 block prefix + PHYPayload (excluding the 4-byte MIC suffix), using the `NwkSKey` as the
key.

### Rust Ecosystem Answer

**RustCrypto** is the authoritative source. Two crates compose to produce the exact
primitive:

| Crate | Version (as of training) | Role | Notes |
|---|---|---|---|
| `aes` | 0.8.x | AES-128 block cipher | RustCrypto; no-std; constant-time |
| `cmac` | 0.7.x | CMAC mode wrapper | RustCrypto; builds on `aes`; implements `digest::Mac` |

Usage pattern:
```rust
use aes::Aes128;
use cmac::{Cmac, Mac};

let mut mac = Cmac::<Aes128>::new_from_slice(nwk_s_key).unwrap();
mac.update(&b0_block);
mac.update(&phy_payload_without_mic);
let result = mac.finalize();
let mic = &result.into_bytes()[..4];
```

Both crates are `no_std` compatible, have no system dependencies, cross-compile cleanly to
armv7/aarch64, and are widely used across the Rust IoT/embedded ecosystem.

**Alternative — `lorawan-encoding` crate**
The `NewAE/lorawan` project (crates.io: `lorawan-encoding`) provides full LoRaWAN frame
encode/decode including MIC verification as an integrated operation. It uses the same
RustCrypto primitives internally. If Maverick ever needs full frame parsing (FPort, FOpts,
payload decrypt), this crate is worth evaluating. For the immediate v1 need (MIC check on
already-parsed uplinks), direct `aes` + `cmac` is simpler and avoids a larger dependency.

**FCnt 32-bit note:** The MIC B0 block includes the full 32-bit FCnt. Since Maverick
currently stores FCnt as 16-bit in `SessionSnapshot::uplink_frame_counter`, the MIC
implementation depends on the FCnt 32-bit fix also landing first. These two Active
requirements must ship together.

### Recommendation
Add `aes = "0.8"` and `cmac = "0.7"` to `maverick-core` (or `maverick-domain` as a
crypto-primitive utility). Implement MIC verification inside
`LoRaWAN10xClassA::validate_uplink`, requiring `NwkSKey` to be available in `SessionSnapshot`
(currently absent — schema change needed). Treat FCnt 32-bit fix as a prerequisite.

---

## Process Supervision

### Problem
`maverick-edge` has no supervisor. If the process panics or is OOM-killed, it stays down
until an operator intervenes. The v1 requirement is: LNS core auto-restarts on crash, never
stays down.

### The Right Answer: systemd

For a Linux-only, single-binary service, **systemd is the correct and complete solution**.
No Rust crate is needed for supervision itself. The existing installer
(`scripts/install-linux.sh`) already provisions the service; it just needs the restart
directives.

**Minimal unit file additions:**

```ini
[Service]
Restart=always
RestartSec=2s
StartLimitIntervalSec=60
StartLimitBurst=5
```

- `Restart=always` covers crashes, OOM kills, non-zero exits, and signal deaths.
- `RestartSec=2s` prevents tight restart loops from burning CPU.
- `StartLimitBurst=5` / `StartLimitIntervalSec=60` prevents infinite flapping (systemd backs
  off after 5 failures in 60 s and sends an alert rather than looping forever).

**Additional hardening worth including:**

```ini
[Service]
MemoryMax=256M          # Hard ceiling — prevents OOM from cascading to system
OOMScoreAdjust=-500     # Deprioritise kernel OOM killer targeting maverick-edge
StandardOutput=journal
StandardError=journal
SyslogIdentifier=maverick-edge
```

### Tokio-Level Supervision (in-process watchdog)

For the ingest loop itself, Maverick already has `run_radio_ingest_supervised` which wraps
the GWMP loop in a retry/backoff pattern. This is correct and complementary to systemd: the
in-process supervisor handles transient I/O errors (socket read failures, parse errors)
without restarting the whole process; systemd handles process-level crashes.

No additional Rust crate is needed. `tokio::time::sleep` + exponential backoff (already
present in `ResilientRadioTransport`) covers the in-process case.

**Relevant crate — `sd-notify` (optional)**

| Crate | Role | Notes |
|---|---|---|
| `sd-notify` | Send `READY=1` and `WATCHDOG=1` pings to systemd | Enables `Type=notify` units and watchdog kill-restart; optional but recommended for production robustness |

With `sd-notify`, the unit can use `WatchdogSec=30` — if `maverick-edge` stops sending
watchdog pings (e.g., stuck in a deadlock rather than crashed), systemd kills and restarts
it. This catches hung-process scenarios that `Restart=always` alone misses.

### Recommendation
1. Update `scripts/install-linux.sh` and the unit file template with `Restart=always` +
   `RestartSec=2s` + memory limits.
2. Add `sd-notify = "0.4"` as an optional dependency in `maverick-runtime-edge`; send
   `READY=1` after successful port bind and `WATCHDOG=1` in the ingest loop heartbeat.
3. No separate supervisor process (supervisord, s6, etc.) — systemd is already on every
   supported target.

---

## Local IPC (Extension System)

### Problem
Output plugins (HTTP forwarder, MQTT bridge, cloud sync, web dashboard) must run as separate
processes. They need a stable local API surface to query `maverick-edge` (get uplinks, get
device list, subscribe to new uplinks) without coupling to internal crate structures. The
TUI already uses the subprocess model; the question is the wire protocol for a longer-lived
IPC channel.

### Options Analysis

| Mechanism | Latency | Complexity | Cross-arch | Framing | Best for |
|---|---|---|---|---|---|
| Unix domain socket + newline-JSON | Very low | Low | Yes (Linux-only is fine) | Manual but trivial | Streaming events + request/reply |
| Local HTTP (TCP loopback) | Low | Medium | Yes | HTTP/JSON — standard | REST-style query API |
| Named pipe (FIFO) | Very low | Low | Yes | Manual | One-way event stream only |
| D-Bus | Low | High | Yes | Schema-defined | System integration; overkill here |
| gRPC (local) | Low | High | Yes | Protobuf | Typed contracts; overkill for v1 |
| Shared SQLite (read-only) | N/A | Very low | Yes | None needed | Read-only extension query |

### Recommendation: Two-tier approach

**Tier 1 — Unix domain socket for live event streaming (new uplinks)**
Extensions that need real-time uplink events subscribe to a Unix socket that `maverick-edge`
publishes to. Protocol: newline-delimited JSON (`serde_json` already present). Each line is
a `SyncEventV1`-shaped JSON object (the contract crate `maverick-extension-contracts`
already defines this schema — use it directly).

**Tier 2 — Local HTTP API for query/management**
Extensions that need to query historical uplinks, device lists, or trigger config changes
call a local HTTP server bound to `127.0.0.1:17001` (or a configurable port). This is
already implicit in the TUI's pattern of running `maverick-edge` subcommands. Formalising
it as a local HTTP server (`axum` or `warp`) provides a stable contract.

**Recommended Rust crates:**

| Crate | Role | Notes |
|---|---|---|
| `tokio::net::UnixListener` | Unix socket server | Already in tokio full; no extra dep |
| `axum` | Local HTTP API server | Minimal, tokio-native; `0.7.x` stable; widely used |
| `serde_json` | JSON framing for both tiers | Already present in workspace |
| `tower` | Middleware for the axum stack | Transitively pulled by axum |

**Why not gRPC or D-Bus:** Both require schema compilation tooling and add significant
build-time complexity incompatible with the project's simple cross-compilation setup.

**Why not shared SQLite (read-only extensions):** Extensions reading the SQLite file directly
couples them to the schema. Schema migrations in `maverick-edge` would silently break
extensions. The IPC surface is the right decoupling point.

**Unix socket path convention:** `/var/run/maverick/events.sock` (created by `maverick-edge`
at startup, permissions `0660`, group `maverick`). Extensions run as the same user or group.

---

## Confidence Levels

| Area | Confidence | Reasoning |
|---|---|---|
| AES-128 CMAC via RustCrypto (`aes` + `cmac`) | **HIGH** | RustCrypto is the definitive Rust cryptography project; these crates have been stable for years, are widely deployed in embedded/IoT Rust, and their API is well-documented. The LoRaWAN MIC construction is specified exactly in the spec. |
| FCnt 32-bit prerequisite for MIC | **HIGH** | This is a logical dependency from the LoRaWAN spec: B0 block uses full 32-bit FCnt; not verifiable without it. |
| systemd `Restart=always` for supervision | **HIGH** | This is documented Linux operational practice. The installer already uses systemd. `sd-notify` crate API is stable. |
| `sd-notify` watchdog integration | **MEDIUM** | Crate is stable and used in production Rust services, but exact API version numbers are from training data (unverified against crates.io at time of writing). |
| Unix socket + axum for extension IPC | **HIGH** | Both are tokio-native, actively maintained, and match the project's existing patterns (tokio already in workspace, serde_json already in workspace). |
| No pure-Rust SX1302/SX1303 HAL | **MEDIUM** | Training data confirms this gap as of mid-2025. A new crate may have appeared since. Treat as HIGH-confidence "likely gap" but verify before starting the SPI adapter phase. |
| FFI `libloragw` approach for SPI | **MEDIUM** | This is the correct architectural approach (used by ChirpStack, BasicStation, RAK SDKs), but the specific `bindgen` integration for Maverick's cross-compilation pipeline has not been prototyped and may surface surprises on armv7. |
| `lorawan-encoding` crate as MIC alternative | **MEDIUM** | Crate exists and is actively maintained (NewAE organization); exact version and API surface unverified at time of writing. |

---

## Gaps / Unknowns

1. **SX1302 pure-Rust driver gap needs live verification** — Search crates.io and GitHub for
   `sx1302`, `sx1303`, `lora-concentrator` Rust crates before starting the SPI adapter
   phase. A pure-Rust SPI implementation may have emerged since mid-2025.

2. **`NwkSKey` is absent from `SessionSnapshot`** — MIC verification requires the Network
   Session Key to be stored per device. This is a schema change to both `maverick-domain`
   (add field to `SessionSnapshot`) and `maverick-adapter-persistence-sqlite` (migration to
   add column). The `lns-config.toml` format must also accept `nwk_s_key` per device. Scope
   this before the MIC phase.

3. **`aes` + `cmac` exact version compatibility** — Verify that the versions available on
   crates.io at implementation time compose correctly (RustCrypto crates use a trait tower
   where `cmac` depends on a specific `aes` minor version). Check the RustCrypto
   `cmac` `Cargo.toml` for the pinned `aes` dependency before adding both.

4. **`axum` vs `warp` choice for local HTTP** — `axum` is the current recommended choice
   (tokio team maintains it, warp is in maintenance mode), but verify `axum 0.7.x` cross-
   compiles cleanly to armv7 before committing. It pulls `hyper 1.x` which has had
   compilation surface changes.

5. **Unix socket path permissions on Raspberry Pi OS** — The `/var/run/maverick/` path
   convention needs to be validated against the actual filesystem layout on Raspberry Pi OS
   (Bookworm). systemd's `RuntimeDirectory=maverick` directive handles this automatically
   if the unit is properly configured.

6. **`libloragw` cross-compilation on CI** — The GitHub Actions release pipeline
   (`rust:1-bookworm` container) does not currently include Semtech's `libloragw` build
   dependencies (libusb, etc.) or the C cross-compiler headers needed for armv7. This will
   require extending the release container if Option A (FFI) is chosen for the SPI adapter.
