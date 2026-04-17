# Phase 8 Hardware Testing - Visibility Report

**Date:** 2026-04-17
**Platform:** Raspberry Pi 4 Model B Rev 1.5 (Debian GNU/Linux 13)
**Maverick Version:** Built from source (commit synced via rsync)

## Executive Summary

**Critical Finding:** RAK LoRa HAT (SX1302/3) is NOT physically attached to the RPi. No SPI devices detected.

## What Works

### Plan 08-A: Hardware Testing Infrastructure

| Task | Status | Notes |
|------|--------|-------|
| SSH Connectivity | PASS | Key-based auth works, no password prompt |
| Board Detection | PASS | RPi 4 Model B detected |
| SPI Devices | FAIL | No `/dev/spidev*` devices - HAT not attached |
| Maverick Build | PASS | Builds successfully in ~7m40s |
| Maverick Binary | PASS | All CLI commands work |
| Config Validation | PASS | Fixed missing `activation_mode` field |

### Integration Tests

| Test Suite | Status | Details |
|------------|--------|---------|
| class_a_downlink | 6/6 PASS | Confirmed uplink, ACK, RX timing all work |
| operator_local_gateway_e2e | 1/1 PASS | End-to-end flow works |
| persistence_sqlite | 6/6 PASS | All DDL and persistence tests pass |
| radio_transport_resilience | 4/4 PASS | UDP downlink, circuit recovery work |
| smoke | 3/3 PASS | Region parse, policy serialize, JSON roundtrip |
| watchdog | 4/6 FAIL | Flaky tests - race condition in socket setup |

**Total: 24/26 integration tests pass (92%)**

### Additional Verified Features

- `maverick-edge probe` - Works, shows gwmp_udp backend
- `maverick-edge health` - Shows degraded (expected without radio)
- `maverick-edge status` - Works, shows ~4GB memory
- `maverick-edge config validate/load/show` - All work after config fix

## What Doesn't Work

### Blocking Issues

1. **No SPI/LoRa Hardware** - `/dev/spidev*` does not exist
   - RAK LoRa HAT is not attached to the RPi
   - Cannot test SX1302/3 radio communication
   - Cannot test actual RF transmit/receive

2. **Watchdog Test Flakiness** - 2 tests fail intermittently
   - Race condition: 10ms sleep insufficient for socket setup
   - Tests pass/fail non-deterministically
   - Not a functional bug - test infrastructure issue

3. **LNS Config Schema Mismatch**
   - Original config missing `activation_mode` field
   - Fixed by adding `activation_mode = "abp"` to device entries
   - Schema validation should require this field

### Plans Requiring Hardware (Cannot Execute)

Due to missing SPI/LoRa hardware, the following plans cannot be executed:
- **08-B**: Real Uplink Test Vectors with Known MIC (hardware SPI required)
- **08-C**: Downlink Testing (requires RX1/RX2 timing hardware)
- **08-D**: SPI Radio Full Verification (requires SX1302/3)
- **08-E**: Stress Test (requires real radio)
- **08-F**: TUI Menus (could test but HAT display also missing)
- **08-G**: Extension Discovery (partially testable without hardware)
- **08-H**: End-to-End Flow (requires radio)
- **08-J**: Performance Metrics (requires real radio)

## Visibility Report

| Component | Status | Details |
|-----------|--------|---------|
| SSH & Connectivity | VISIBLE | Works fully |
| Build System | VISIBLE | Rust 1.95.0, builds cleanly |
| CLI Commands | VISIBLE | All commands respond |
| Config System | VISIBLE | TOML parse, validate, load all work |
| SQLite Persistence | VISIBLE | All persistence tests pass |
| GWMP UDP Ingest | VISIBLE | UDP transport works |
| Radio SPI | NOT VISIBLE | Hardware not attached |
| RF Radio | NOT VISIBLE | No radio to test |

## Required Actions

1. **Attach RAK LoRa HAT** to enable hardware testing
2. **Fix watchdog test race condition** - increase sleep to 100ms or use sync barrier
3. **Update LNS config schema** - make `activation_mode` required
4. **Re-run Phase 8** after HAT attachment

## Severity Assessment

- **Hardware Missing**: BLOCKING - Cannot complete hardware testing phase
- **Watchdog Flakiness**: WARNING - Test infrastructure only
- **Config Schema**: WARNING - Should document required fields
