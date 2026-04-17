---
phase: "08"
plan: "08-F"
subsystem: hardware_testing
type: testing
status: pending
wave: 3
depends_on: ["08-B", "08-C", "08-D"]
autonomous: false
requirements_addressed: []
---

# Plan 08-F: TUI Menus Verification

## Objective

Verify TUI functionality on RAK Pi display. Confirm all menus render and function correctly.

## Tasks

### Task 1: TUI Startup Verification

**Action:**
```bash
ssh pi@rak.local "cd /home/pi/maverick && ./target/release/maverick-edge tui --help"
```

**Acceptance Criteria:**
- TUI command available
- Help text displays
- No immediate crashes

### Task 2: TTY Rendering Test

**Action:**
1. SSH to RAK Pi with screen/tmux
2. Launch TUI: `./target/release/maverick-edge tui`
3. Verify menus render without corruption

**Acceptance Criteria:**
- Dashboard visible
- All menu items readable
- No encoding issues

### Task 3: Device List Menu

**Action:**
1. Navigate to device list in TUI
2. Verify registered devices shown
3. Check last-seen timestamps

**Acceptance Criteria:**
- Device count matches database
- DevAddr and DevEUI displayed
- Timestamps update on new uplinks

### Task 4: Add Device via TUI

**Action:**
1. Navigate to "Add Device" in TUI
2. Enter test device credentials
3. Submit and verify

**Acceptance Criteria:**
- Device added successfully
- New device appears in list
- Subsequent uplinks from device accepted

### Task 5: Remove Device via TUI

**Action:**
1. Navigate to device in list
2. Select remove/delete option
3. Confirm removal

**Acceptance Criteria:**
- Device removed from list
- Subsequent uplinks rejected (not in database)

---
*Plan: 08-F*
*Phase: 08-hardware-testing-rak-pi*
