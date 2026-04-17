---
phase: 5
name: TUI Device Management
verification_date: 2026-04-16
status: complete_with_gaps
---

# Phase 5: TUI Device Management - Verification

## Requirements Verification

### DEV-01: Add device via TUI

**Status**: PARTIAL

**Verification Evidence**:
- `lns_wizard.rs`: `run_devices_wizard()` function exists
- `menu_lorawan.rs`: Option 8 maps to `run_devices_wizard(cfg)`
- TUI menu: `Guided — devices (add/edit/remove, save, optional load)`

**Gap**: Formal verification not completed. Implementation exists but no automated test validates the wizard flow.

**Evidence**: Wizard implementation present, operator can add devices through guided flow

---

### DEV-02: List devices with last-seen and uplink-count

**Status**: SATISFIED

**Verification Evidence**:
- `LnsDeviceListRow` extended with `last_seen_timestamp` and `uplink_count` fields
- `lns_list_devices()` query updated with `LEFT JOIN on sessions` table and subquery for uplink count
- Commit: `6414b82`

**Test Method**:
```bash
maverick-edge config list-devices
# Returns JSON with last_seen_timestamp and uplink_count per device
```

---

### DEV-03: Remove device via TUI

**Status**: PARTIAL

**Verification Evidence**:
- Device removal workflow: `lns-config.toml` editing + `config load`
- TUI menu option 8 (devices wizard) includes remove functionality
- No standalone `device remove` CLI command explicitly added

**Gap**: Direct `device remove` CLI command not implemented. Relies on TOML edit + config reload workflow.

**Workaround**: Operator uses wizard (option 8) → edit → remove device → save → config load

---

### DEV-04: lns-config.toml import

**Status**: SATISFIED

**Verification Evidence**:
- `apply_lns_config()` in `lns_ops.rs` handles device sync
- TUI menu option 6: `Load — apply file to SQLite`
- Config load workflow: validate → preview → confirm → apply

**Test Method**:
```bash
# Option 6 in lorawan menu loads lns-config.toml to SQLite
maverick-edge config load --config-path /etc/maverick/lns-config.toml
```

---

### DEV-05: Promote pending devices

**Status**: SATISFIED

**Verification Evidence**:
- `lns_list_pending()` returns pending devices
- `lns_approve_device()` promotes pending → registered
- TUI menu option 5: `List pending DevAddrs (JSON)`
- Commit: `6414b82`

**Test Method**:
```bash
maverick-edge config list-pending  # Shows unknown DevAddrs
# Use wizard option 9 (autoprovision) to promote
```

---

### CORE-03: Hardware probe on startup

**Status**: PARTIAL

**Verification Evidence**:
- `probe.rs` in `maverick-runtime-edge`: `HardwareCapabilities::probe()` exists
- `doctor.rs`: `probe_edge_capabilities()` runs probe and displays in dashboard
- TTY startup screen shows hardware info (CPU arch, RAM, storage)
- Probe runs automatically on TUI startup (via `edge_runner`)

**Gap**: Formal verification not completed. Implementation exists and is visible in doctor dashboard.

---

## Summary

| Requirement | Status | Evidence |
|-------------|--------|----------|
| DEV-01 | PARTIAL | Wizard exists, not formally verified |
| DEV-02 | SATISFIED | last_seen + uplink_count fields, commit 6414b82 |
| DEV-03 | PARTIAL | TOML edit + reload workflow, no direct CLI remove |
| DEV-04 | SATISFIED | apply_lns_config(), menu option 6 |
| DEV-05 | SATISFIED | lns_list_pending(), lns_approve_device(), menu option 5 |
| CORE-03 | PARTIAL | Probe runs, visible in dashboard, not formally verified |

**Phase Status**: COMPLETE (with partial requirements: DEV-01, DEV-03, CORE-03)

## Tech Debt

1. No formal TUI wizard tests (DEV-01 gap)
2. No direct `device remove` CLI command (DEV-03 workaround)
3. Hardware probe not formally verified end-to-end (CORE-03 gap)
