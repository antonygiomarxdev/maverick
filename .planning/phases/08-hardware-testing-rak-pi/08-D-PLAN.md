---
phase: "08"
plan: "08-D"
subsystem: hardware_testing
type: testing
status: pending
wave: 2
depends_on: ["08-A"]
autonomous: false
requirements_addressed: []
---

# Plan 08-D: SPI Radio (SX1302/3) Full Verification

## Objective

Verify SPI communication with SX1302/3 concentrator. Confirm full receive and transmit path works.

## Context

Phase 2 created the SPI adapter but libloragw RX integration was marked as placeholder. This test determines the current state.

## Tasks

### Task 1: SPI Initialization Test

**Action:**
```bash
ssh pi@rak.local "cd /home/pi/maverick && sudo ./target/release/maverick-edge --radio spi 2>&1 | head -50"
```

**Acceptance Criteria:**
- SPI device opened successfully
- Concentrator reset sequence executed
- No SPI communication errors in first 10 seconds

### Task 2: Receive Packets via SPI

**Action:**
1. Have a LoRa device send uplinks
2. Observe maverick-edge logs for received packets
3. Verify packets appear in SQLite

**Acceptance Criteria:**
- Packets received via SPI (not UDP)
- Log shows "SPI RX" not "UDP RX"
- Packets persisted to SQLite

### Task 3: Transmit Packets via SPI

**Action:**
1. Queue downlink for test device
2. Trigger downlink transmission
3. Observe SPI TX activity

**Acceptance Criteria:**
- SPI write operations observed (via strace or logs)
- Concentrator TX indicator lights
- Downlink frame transmitted on correct frequency

### Task 4: SX1302 vs SX1303 Detection

**Action:**
```bash
ssh pi@rak.local "cat /sys/firmware/devicetree/base/model 2>/dev/null || echo 'Unknown board'"
```

**Acceptance Criteria:**
- Board model identified
- Correct concentrator driver loaded (sx1302 vs sx1303)
- No driver mismatch errors

---
*Plan: 08-D*
*Phase: 08-hardware-testing-rak-pi*
