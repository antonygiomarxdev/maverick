---
phase: "08"
plan: "08-A"
subsystem: hardware_testing
type: testing
status: completed
wave: 1
depends_on: []
autonomous: false
requirements_addressed: []
---

# Plan 08-A: Hardware Testing Infrastructure

## Objective

Set up hardware testing environment on RAK Pi, verify SSH connectivity, and establish baseline.

## Context

Phase 8 is a hardware testing phase targeting the RAK LoRa HAT (SX1302/3) on Raspberry Pi. This plan establishes the testing infrastructure and connectivity.

## Tasks

### Task 1: Verify SSH Connectivity to RAK Pi

**Action:**
```bash
ssh -o ConnectTimeout=10 pi@rak.local "echo 'SSH OK' && uname -a && cat /etc/os-release"
```

**Acceptance Criteria:**
- SSH connection succeeds without password prompt (key-based auth configured)
- Output shows Raspberry Pi OS or similar
- `ls /dev/spidev*` shows SPI devices present

### Task 2: Check SPI Devices on RAK Pi

**Action:**
```bash
ssh pi@rak.local "ls -la /dev/spidev* && ls -la /dev/lora* 2>/dev/null || echo 'No lora device'"
```

**Acceptance Criteria:**
- `/dev/spidev0.0` and `/dev/spidev0.1` exist
- SPI device permissions allow access

### Task 3: Check Maverick Installation on RAK Pi

**Action:**
```bash
ssh pi@rak.local "cd /home/pi/maverick && git status && cargo build --release 2>&1 | tail -20"
```

**Acceptance Criteria:**
- Maverick source code is present
- Cargo build completes or fails with visible errors

### Task 4: Verify concentrator firmware version

**Action:**
```bash
ssh pi@rak.local "sudo /opt/ rak/spi_test 2>/dev/null || echo 'No spi_test tool'"
```

**Acceptance Criteria:**
- Concentrator firmware version retrieved or noted as unavailable

---
*Plan: 08-A*
*Phase: 08-hardware-testing-rak-pi*
