---
phase: 03.1-class-a-downlink
status: partial
verification_date: 2026-04-17
verified_by: tbd
---

# Phase 3.1: Class A Downlink — Verification

## Success Criteria Status

| # | Criterion | Status | Evidence |
|---|-----------|--------|----------|
| 1 | When a downlink is queued for a device, Maverick transmits it in the RX1 window (1 second after uplink end) using the hardware timestamp from the concentrator | ⚠️ PARTIAL | DownlinkScheduler implemented with RX1_DELAY_MS=1000ms; NOT YET INTEGRATED into ingest loop |
| 2 | If RX1 transmission fails, Maverick falls back and transmits in the RX2 window (2 seconds after uplink end) | ⚠️ PARTIAL | RX2 scheduling implemented in DownlinkScheduler; NOT YET INTEGRATED |
| 3 | A confirmed uplink receives a downlink with the ACK flag set | ⚠️ PARTIAL | ACK handling logic not yet implemented in ingest loop |
| 4 | Queued downlinks written to SQLite before transmission attempt — a process restart does not lose pending downlinks | ✅ VERIFIED | SQLite schema and DownlinkRepository port implemented; integration tests passing |
| 5 | A device sending LinkCheckReq in FOpts receives a LinkCheckAns in the next downlink with correct margin and gateway count | ⚠️ PARTIAL | LinkCheckAns struct implemented; NOT YET INTEGRATED into ingest loop |

## Verification Evidence

### Criterion 4
- `downlink_queue` table with indexes exists in SQLite schema
- `DownlinkRepository` port methods: `enqueue`, `dequeue_oldest`, `mark_transmitted`, `mark_failed`, `get_pending_for_dev`
- Integration tests for queue persistence passing

### DownlinkScheduler
- Implemented in `crates/maverick-runtime-edge/src/ingest/downlink.rs`
- RX1 at +1000ms, RX2 at +2000ms per LoRaWAN Class A spec
- Designed as generic over DownlinkRepository and RadioTransport traits

## Integration Gaps

| Component | Status |
|-----------|--------|
| DownlinkScheduler wired into runtime startup | ❌ Missing |
| ACK flag detection in ingest loop | ❌ Missing |
| LinkCheckAns transmission in downlink path | ❌ Missing |
| RX1/RX2 actual transmission via SPI | ❌ Missing (requires hardware) |

## Validation Audit 2026-04-17

| Metric | Count |
|--------|-------|
| Gaps found | 5 |
| Resolved | 0 |
| Escalated | 5 |

## Hardware Testing Required

Full verification of criteria 1, 2, 3, 5 requires:
- Physical RAK Pi with SX1302/3 concentrator
- Real device sending uplinks
- Ability to observe RX1/RX2 downlink transmissions
- Packet capture to verify LinkCheckAns format

---
*Verification created: 2026-04-17*
*Validation audit: 2026-04-17*
