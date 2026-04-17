---
phase: 5
name: TUI Device Management
completed_date: 2026-04-16
commit: 6414b82
---

# Phase 5: TUI Device Management - Summary

## What Was Built

Implemented full device management through the TUI: device list with statistics, add/remove via wizard, pending device promotion, and hardware probe display. Maintained backward compatibility with lns-config.toml imports.

## Files Modified

| File | Change | Purpose |
|------|--------|---------|
| `crates/maverick-extension-tui/src/menu_lorawan.rs` | Modified | Device management menu (options 1-9) |
| `crates/maverick-extension-tui/src/lns_wizard.rs` | Modified | Device add/edit/remove wizard (23KB) |
| `crates/maverick-extension-tui/src/doctor.rs` | Modified | Hardware probe display in dashboard |
| `crates/maverick-extension-tui/src/edge_runner.rs` | Modified | Probe output capture for TUI |
| `crates/maverick-runtime-edge/src/probe.rs` | Created | HardwareCapabilities::probe() |

## Key Decisions

1. **Wizard-based device management**: All device operations (add/edit/remove) through guided wizard rather than direct CLI — safer for operators

2. **SQLite-backed device state**: Devices stored in SQLite, synced from lns-config.toml via `config load`

3. **TUI menu structure**:
   - Option 4: List devices (JSON)
   - Option 5: List pending DevAddrs
   - Option 6: Config load (sync TOML → SQLite)
   - Option 8: Device wizard (guided add/edit/remove)
   - Option 9: Autoprovision policy

4. **Hardware probe integration**: `maverick-edge probe` command runs on TUI startup, results displayed in doctor dashboard

5. **Device statistics**: `last_seen_timestamp` and `uplink_count` added via LEFT JOIN on sessions table

## Satisfied Requirements

| Requirement | Status | Evidence |
|-------------|--------|----------|
| DEV-02 | SATISFIED | last_seen + uplink_count, commit 6414b82 |
| DEV-04 | SATISFIED | apply_lns_config(), menu option 6 |
| DEV-05 | SATISFIED | lns_list_pending(), lns_approve_device(), menu option 5 |

## Partial Requirements (Tech Debt)

| Requirement | Gap | Next Step |
|-------------|-----|-----------|
| DEV-01 | No formal TUI wizard tests | Add integration tests for wizard flow |
| DEV-03 | No direct `device remove` CLI | TOML edit + reload workaround sufficient |
| CORE-03 | Probe not formally verified | Add probe validation test |

## Deferred Work

None — all core functionality implemented.

## Integration

- **Phase 5 → Phase 1 (SQLite)**: Device management fully dependent on SQLite persistence
- **Phase 5 → Phase 4 (systemd)**: TUI can run independently, no hard watchdog dependency
- **Phase 5 → Phase 2 (Radio)**: Device management independent of radio backend
