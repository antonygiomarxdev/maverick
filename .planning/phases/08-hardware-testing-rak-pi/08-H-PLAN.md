---
phase: "08"
plan: "08-H"
subsystem: hardware_testing
type: testing
status: pending
wave: 3
depends_on: ["08-B", "08-C", "08-D"]
autonomous: false
requirements_addressed: []
---

# Plan 08-H: End-to-End Flow Verification

## Objective

Verify complete data path from LoRa radio to SQLite persistence.

## Tasks

### Task 1: Complete Uplink Flow

**Action:**
1. Have test device send uplink with payload "hello"
2. Trace path: Device → Radio → SPI → LNS → SQLite
3. Query SQLite for received frame

**Acceptance Criteria:**
- Uplink received at radio (SPI logs)
- Frame parsed by LNS (packet forwarder logs)
- MHDR, MACPayload, MIC extracted correctly
- Payload decrypted and logged
- Record inserted in SQLite `uplinks` table

### Task 2: Complete Downlink Flow

**Action:**
1. Queue downlink with payload "response"
2. Trigger transmission
3. Verify device receives downlink

**Acceptance Criteria:**
- Downlink queued in SQLite
- TX scheduled at RX1 or RX2
- SPI TX command sent
- Device receives and ACKs

### Task 3: Round-Trip Timing

**Action:**
1. Send uplink with timestamp T0
2. Queue immediate downlink
3. Measure time until device receives (T1)
4. Calculate total round-trip latency

**Acceptance Criteria:**
- Round-trip latency measured
- Latency under 3 seconds (TX at RX1)
- Both uplinks and downlinks logged

---
*Plan: 08-H*
*Phase: 08-hardware-testing-rak-pi*
