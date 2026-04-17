---
phase: "08"
plan: "08-C"
subsystem: hardware_testing
type: testing
status: pending
wave: 2
depends_on: ["08-A"]
autonomous: false
requirements_addressed: []
---

# Plan 08-C: Downlink Testing (Confirmed + Unconfirmed)

## Objective

Verify Class A downlink functionality: RX1/RX2 timing, ACK flag handling, confirmed vs unconfirmed uplink behavior.

## Context

Phase 3.1 implemented DownlinkScheduler but integration into the ingest loop is pending. This test validates what works and what needs fixing.

## Tasks

### Task 1: Test Unconfirmed Uplink → No ACK

**Action:**
1. Send unconfirmed uplink from test device
2. Observe logs for downlink scheduling
3. Verify no ACK is sent

**Acceptance Criteria:**
- Unconfirmed uplink logged without ACK flag
- No downlink scheduled (or empty queue after RX2)

### Task 2: Test Confirmed Uplink → ACK Downlink

**Action:**
1. Send confirmed uplink (MType=2) from test device
2. Observe logs for downlink scheduling in RX1
3. Verify ACK flag set in downlink

**Acceptance Criteria:**
- Confirmed uplink triggers downlink in RX1 window
- Downlink has ACK flag set (FPending if queue has more)
- Log shows "TX @ RX1" or "TX @ RX2"

### Task 3: Test RX1/RX2 Timing

**Action:**
1. Send uplink with precise timestamp
2. Measure time to downlink transmission
3. Verify RX1 at ~1s, RX2 at ~2s

**Acceptance Criteria:**
- RX1 occurs at 1000ms ± 100ms after uplink end
- RX2 occurs at 2000ms ± 100ms after uplink end
- Timing logged with microsecond precision

### Task 4: Test Downlink Queue Persistence

**Action:**
1. Queue downlink for device
2. Restart maverick-edge process
3. Verify queued downlink still present

**Acceptance Criteria:**
- Pending downlinks survive process restart
- SQLite `downlink_queue` table retains entries
- Downlink transmitted after restart

---
*Plan: 08-C*
*Phase: 08-hardware-testing-rak-pi*
