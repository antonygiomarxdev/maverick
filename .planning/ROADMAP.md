# Roadmap: Maverick LNS

## Overview

Maverick ships in five phases that build on each other from the ground up. Phase 1 makes the LNS correct — real MIC verification, 32-bit FCnt, session keys, and the reliability fixes that make everything else safe to build on. Phase 2 abstracts the radio layer and plugs in direct SPI hardware, eliminating the packet forwarder dependency. Phase 3 adds Class A downlink so devices can receive commands. Phase 4 wraps the process in systemd supervision so it self-heals after crashes. Phase 5 brings TUI device management so operators never have to touch a TOML file to register or remove a device.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [x] **Phase 1: Protocol Correctness** - MIC verification, 32-bit FCnt, session keys, region fix, and reliability groundwork
- [ ] **Phase 2: Radio Abstraction & SPI** - UplinkSource port trait, direct SPI adapter, hardware registry, UDP hardening
- [ ] **Phase 3: Class A Downlink** - RX1/RX2 window scheduling, ACK flag, persistent downlink queue
- [ ] **Phase 4: Process Supervision** - systemd Restart=always, watchdog, key-at-rest protection
- [ ] **Phase 5: TUI Device Management** - Add/list/remove devices via terminal UI backed by SQLite

## Phase Details

### Phase 1: Protocol Correctness
**Goal**: Every accepted uplink is cryptographically verified and frame-counted correctly — Maverick behaves as a real LNS, not an open relay
**Depends on**: Nothing (first phase)
**Requirements**: CORE-01, CORE-02, PROT-01, PROT-02, PROT-03, PROT-04, PROT-05, PROT-06, RELI-01, RELI-02, SEC-01
**Success Criteria** (what must be TRUE):
  1. A frame with a tampered MIC is rejected and logged; a frame with a valid MIC is accepted and persisted to SQLite
  2. A device that has sent more than 65535 uplinks continues to have its FCnt correctly reconstructed — session does not break
  3. NwkSKey and AppSKey are stored per session in SQLite and used for MIC computation and payload decryption
  4. Decrypted uplink payload is persisted to SQLite alongside raw frame data
  5. Region inference correctly identifies AU915 and AS923 without being shadowed by US915; UDP bind address defaults to 127.0.0.1
  6. A duplicate uplink frame (same DevAddr + FCnt arriving twice within the dedup window) is discarded — only one copy persisted to SQLite
**Plans**: 6 plans

Plans:
- [x] 01-A-PLAN.md — Domain model + schema foundations (SessionSnapshot keys, UplinkObservation wire_mic/u16 f_cnt, schema migration, UplinkRecord decrypted payload)
- [x] 01-B-PLAN.md — FCnt 32-bit extension + region inference fix (extend_fcnt, MAX_FCNT_GAP, infer_region AU915/AS923 arm ordering, GWMP parser wire_mic)
- [x] 01-C-PLAN.md — MIC verification + AppSKey decryption (IngestUplink execute pipeline, aes 0.9 + cmac 0.8)
- [x] 01-D-PLAN.md — SQLite dedup (is_duplicate port method, UplinkRepository impl, execute dedup check)
- [x] 01-E-PLAN.md — Reliability fixes (.expect() audit in lns_ops.rs, SqlitePersistence::close(), process::exit cleanup)
- [x] 01-F-PLAN.md — UDP bind default + CORE-01 audit (127.0.0.1:17000, zero external HTTP/DNS verification)

### Phase 2: Radio Abstraction & SPI
**Goal**: The ingest loop is radio-agnostic and Maverick can read frames directly from an SX1302/SX1303 concentrator via SPI — no external packet forwarder required
**Depends on**: Phase 1
**Requirements**: RELI-05, RADIO-01, RADIO-02, RADIO-03, RADIO-04, CORE-04
**Success Criteria** (what must be TRUE):
  1. `UplinkSource` port trait is implemented by both the UDP adapter and the new SPI adapter — ingest loop code is unchanged when backend switches
  2. On a Raspberry Pi with RAK LoRa HAT, Maverick reads and persists uplinks directly from the SX1302/SX1303 without a Semtech packet forwarder running
  3. Radio backend (SPI or UDP) is selectable via config file — existing UDP path remains fully functional for dev and simulator use
  4. Hardware compatibility registry lists RAK Pi HAT as verified-supported; ships as a TOML file community can extend without code changes
**Plans**: 4 plans

Plans:
- [x] 02-A-PLAN.md — `UplinkSource` port trait + optional `[radio]` in `lns-config.toml` (RELI-05, RADIO-03 schema)
- [x] 02-B-PLAN.md — `GwmpUdpUplinkSource` + refactor `gwmp_loop` to `next_batch()` (UDP path)
- [ ] 02-C-PLAN.md — `maverick-adapter-radio-spi` + libloragw / feature `spi` + runtime wiring (RADIO-01/02)
- [x] 02-D-PLAN.md — `hardware-registry.toml` + docs (CORE-04, RADIO-04)

### Phase 3: Class A Downlink
**Goal**: Maverick can send downlinks to Class A devices through both RX windows, with confirmed-uplink ACKs, and the downlink queue survives process restarts
**Depends on**: Phase 1
**Requirements**: DWNL-01, DWNL-02, DWNL-03, DWNL-04, DWNL-05, DWNL-06
**Success Criteria** (what must be TRUE):
  1. When a downlink is queued for a device, Maverick transmits it in the RX1 window (1 second after uplink end) using the hardware timestamp from the concentrator
  2. If RX1 transmission fails, Maverick falls back and transmits in the RX2 window (2 seconds after uplink end)
  3. A confirmed uplink receives a downlink with the ACK flag set
  4. Queued downlinks written to SQLite before transmission attempt — a process restart does not lose pending downlinks
  5. A device sending LinkCheckReq in FOpts receives a LinkCheckAns in the next downlink with correct margin and gateway count
**Plans**: TBD

### Phase 4: Process Supervision
**Goal**: The maverick-edge process self-heals after any crash or hang — an operator never has to manually restart the LNS after a failure
**Depends on**: Phase 1
**Requirements**: RELI-03, RELI-04, SEC-02
**Success Criteria** (what must be TRUE):
  1. After a simulated crash (SIGKILL), maverick-edge is automatically restarted by systemd within 2 seconds with no operator action
  2. A hung process (ingest loop stalled, no watchdog pings) is detected and killed by systemd WatchdogSec — then restarted
  3. NwkSKey and AppSKey are not readable as plaintext from the SQLite schema by an unprivileged user
**Plans**: TBD

### Phase 5: TUI Device Management
**Goal**: An operator can add, inspect, and remove devices entirely through the terminal UI — no manual TOML editing required for routine device provisioning
**Depends on**: Phase 2, Phase 4
**Requirements**: DEV-01, DEV-02, DEV-03, DEV-04, DEV-05, CORE-03
**Success Criteria** (what must be TRUE):
  1. Operator opens TUI and adds a new device (DevEUI, DevAddr, keys, region, application) — device is immediately active without restarting the LNS
  2. Device list screen shows every registered device with last-seen timestamp and uplink count
  3. Operator removes a device via TUI — subsequent frames from that DevAddr are rejected
  4. A legacy lns-config.toml import still provisions all devices correctly — no breaking change to existing bulk workflow
  5. Autoprovision-pending devices (unknown DevAddr frames) visible in TUI — operator can promote one to registered device in a single action
  6. Hardware probe (CPU arch, available RAM, storage) runs automatically on startup and its results are visible in the TUI
**Plans**: TBD
**UI hint**: yes

## Progress

**Execution Order:**
Phases execute in numeric order: 1 → 2 → 3 → 4 → 5

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Protocol Correctness | 6/6 | Complete | 2026-04-16 |
| 2. Radio Abstraction & SPI | 3/4 | In progress (SPI adapter pending) | - |
| 3. Class A Downlink | 0/TBD | Not started | - |
| 4. Process Supervision | 0/TBD | Not started | - |
| 5. TUI Device Management | 0/TBD | Not started | - |
