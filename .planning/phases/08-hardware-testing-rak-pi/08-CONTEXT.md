---
phase: 08-hardware-testing-rak-pi
status: ready_for_planning
created: 2026-04-16
source: user_provided
---

# Phase 8: Hardware Testing (RAK Pi) — Context

**Gathered:** 2026-04-16
**Status:** Ready for planning
**Source:** User-provided requirements

## Phase Boundary

Hardware testing on real RAK Pi hardware with SX1302/3 LoRa concentrator. This phase validates that the software stack works correctly on actual hardware, identifies gaps, and produces a visibility report.

## Hardware Target

- **Device:** RAK LoRa HAT on Raspberry Pi
- **Concentrator:** SX1302/SX1303
- **SSH Access:** `pi@rak.local`

## Implementation Requirements

### Core Testing Areas

1. **Real uplink test vectors with known MIC**
   - Use LoRaWAN test vectors with pre-computed MICs
   - Verify uplink acceptance/rejection
   - Validate frame counters

2. **Downlink testing (confirmed + unconfirmed)**
   - Test confirmed uplink → ACK downlink
   - Test unconfirmed uplink → no ACK
   - Verify RX1/RX2 timing

3. **SPI radio (SX1302/3) full verification**
   - Verify SPI communication with concentrator
   - Test packet reception from real devices
   - Validate transmit path

4. **Stress test (high volume uplinks)**
   - Send high volume of uplinks
   - Verify no packet loss
   - Monitor memory/CPU

5. **TUI menus verification**
   - Verify TUI renders correctly
   - Test device management menus
   - Check status displays

6. **Extension discovery/health**
   - Verify extensions can be discovered
   - Test extension health reporting
   - Validate extension isolation

7. **End-to-end flow verification**
   - Device → Radio → SPI → LNS → SQLite
   - Verify complete data path

8. **Visibility report: what works / what doesn't**
   - Document successful components
   - Document failing components
   - Prioritize gaps

9. **Performance metrics**
   - Uplinks/second capacity
   - Memory usage under load
   - CPU usage under load
   - Latency measurements

## SSH Access Configuration

```
Host: pi@rak.local
User: pi
Hardware: RAK LoRa HAT + Raspberry Pi
```

## Dependencies

- Phase 7 (Phase 5 Verification & Artifacts) — provides TUI and device management
- Phase 2 (Radio Abstraction & SPI) — provides SPI adapter
- Phase 3.1 (Class A Downlink) — provides downlink scheduling

## Technical Notes

- This is primarily a TESTING/VALIDATION phase, not a development phase
- Focus is on verifying existing implementations work on hardware
- Gap closure from VERIFICATION.md files in prior phases

## Deferred Ideas

None — full scope defined above

---
*Phase: 08-hardware-testing-rak-pi*
*Context gathered: 2026-04-16*
