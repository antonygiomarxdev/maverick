---
phase: "08"
plan: "08-G"
subsystem: hardware_testing
type: testing
status: pending
wave: 3
depends_on: ["08-B", "08-C", "08-D"]
autonomous: false
requirements_addressed: []
---

# Plan 08-G: Extension Discovery and Health

## Objective

Verify extension system: discovery, health reporting, and isolation.

## Tasks

### Task 1: Extension Discovery Test

**Action:**
```bash
ssh pi@rak.local "cd /home/pi/maverick && ./target/release/maverick-edge extensions list"
```

**Acceptance Criteria:**
- Command succeeds
- Built-in extensions listed
- No errors in output

### Task 2: Extension Health Check

**Action:**
```bash
ssh pi@rak.local "cd /home/pi/maverick && ./target/release/maverick-edge health"
```

**Acceptance Criteria:**
- Health report generated
- All core components show status
- Extension status included

### Task 3: Extension Isolation Test

**Action:**
1. Create a simple extension that crashes
2. Verify core process survives
3. Check that crash is logged

**Acceptance Criteria:**
- Extension crash does not kill maverick-edge
- Error logged with extension name
- Health check shows extension as unhealthy

### Task 4: Extension Communication Test

**Action:**
1. Implement a simple HTTP extension
2. Send request to extension endpoint
3. Verify response received

**Acceptance Criteria:**
- Extension receives requests
- Responses delivered correctly
- IPC mechanism functional

---
*Plan: 08-G*
*Phase: 08-hardware-testing-rak-pi*
