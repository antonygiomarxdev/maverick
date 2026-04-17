---
phase: "08"
plan: "08-J"
subsystem: hardware_testing
type: testing
status: pending
wave: 3
depends_on: ["08-E"]
autonomous: false
requirements_addressed: []
---

# Plan 08-J: Performance Metrics

## Objective

Measure and document system performance on RAK Pi hardware.

## Tasks

### Task 1: Baseline Resource Usage

**Action:**
```bash
ssh pi@rak.local "top -bn1 | head -20 && free -m && df -h /"
```

**Acceptance Criteria:**
- CPU usage at idle captured
- Memory total/used/free documented
- Disk usage for SQLite database location

### Task 2: Uplink Throughput Measurement

**Action:**
1. Configure 3 test devices sending every 2 seconds (90 uplinks/minute)
2. Run for 5 minutes
3. Count uplinks in SQLite vs sent

**Acceptance Criteria:**
- Throughput measured in uplinks/second
- No packet loss up to tested rate
- Maximum sustainable rate identified

### Task 3: CPU Under Load

**Action:**
```bash
ssh pi@rak.local "while true; do ./target/release/maverick-edge & PID=$!; sleep 60; top -bn1 -p $PID | tail -5; done"
```

**Acceptance Criteria:**
- CPU usage at 1 uplinks/sec documented
- CPU usage at 10 uplinks/sec documented
- CPU usage at 50 uplinks/sec documented
- Threshold where CPU maxes out identified

### Task 4: Memory Stability

**Action:**
```bash
ssh pi@rak.local "while true; do ./target/release/maverick-edge & PID=$!; sleep 300; ps -o rss= -p $PID; done"
```

**Acceptance Criteria:**
- Memory usage at startup documented
- Memory after 1 hour documented
- Memory after 24 hours documented
- Memory leak detected if any

### Task 5: Latency Measurement

**Action:**
1. Send uplink with timestamp in payload
2. Record time received in LNS logs
3. Calculate radio-to-LNS latency

**Acceptance Criteria:**
- Average latency measured
- P99 latency measured
- Jitter documented

---
*Plan: 08-J*
*Phase: 08-hardware-testing-rak-pi*
