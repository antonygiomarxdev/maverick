---
phase: "08"
plan: "08-B"
subsystem: hardware_testing
type: testing
status: pending
wave: 2
depends_on: ["08-A"]
autonomous: false
requirements_addressed: []
---

# Plan 08-B: Real Uplink Test Vectors with Known MIC

## Objective

Verify uplink processing with LoRaWAN test vectors containing known MIC values. Validate that valid frames are accepted and invalid MICs are rejected.

## Context

LoRaWAN test vectors from the spec allow testing MIC verification without real devices. This test validates the crypto implementation.

## Tasks

### Task 1: Gather LoRaWAN Test Vectors

**Action:**
Research and document known-good LoRaWAN 1.0.x test vectors from:
- LoRaWAN 1.0.x Specification (Section 4.3 MIC computation)
- ChirpStack test vectors
- Existing test vectors in `crates/maverick-integration-tests/`

**Acceptance Criteria:**
- Document at least 3 test vectors with DevAddr, FCnt, AppSKey, NwkSKey, MHDR, MIC
- Each vector has known-valid and known-invalid MIC variant

### Task 2: Create Uplink MIC Test Program

**Action:**
Create a test program that:
1. Constructs uplink frames using test vectors
2. Sends via UDP to localhost:17000 (GWMP format)
3. Verifies acceptance/rejection via logs

**Acceptance Criteria:**
- Test program compiles and runs
- Valid MIC frames accepted (logged)
- Invalid MIC frames rejected (logged)

### Task 3: Run MIC Verification Tests

**Action:**
```bash
cargo test -p maverick-integration-tests test_mic_verification
```

**Acceptance Criteria:**
- All MIC verification tests pass
- Valid MIC accepted
- Invalid MIC rejected with warning log

### Task 4: Hardware SPI MIC Test

**Action:**
```bash
# Run test via SPI on RAK Pi
ssh pi@rak.local "cd /home/pi/maverick && cargo test -p maverick-integration-tests test_spi_mic"
```

**Acceptance Criteria:**
- Test runs on hardware
- Valid MIC frames accepted via SPI
- Log output confirms uplink persistence

---
*Plan: 08-B*
*Phase: 08-hardware-testing-rak-pi*
