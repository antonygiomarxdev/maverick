---
phase: "07"
name: "Phase 5 Verification Gap Closure"
wave: 1
depends_on: []
autonomous: true
requirements_addressed:
  - CORE-03
  - DEV-01
  - DEV-02
  - DEV-03
  - DEV-04
  - DEV-05
files_modified: []
---

## Plan: Phase 5 Verification Gap Closure

### Objective

Create formal VERIFICATION.md and SUMMARY.md for Phase 5 (TUI Device Management) to address audit gaps. DEV-02, DEV-04, DEV-05 are satisfied. CORE-03, DEV-01, DEV-03 are partial.

### Success Criteria

1. **VERIFICATION.md exists** with test evidence for all DEV-xx requirements
2. **SUMMARY.md exists** documenting what was built
3. **Partial requirements** (CORE-03, DEV-01, DEV-03) have gap documentation

### Tasks

#### Task 1: Create VERIFICATION.md for Phase 05

<read_first>
- .planning/v1.0-MILESTONE-AUDIT.md
- .planning/phases/05-tui-device-management/05-PLAN.md
</read_first>

<action>
Create `.planning/phases/05-tui-device-management/05-VERIFICATION.md` documenting:

1. **DEV-01 (Add device via TUI)**: PARTIAL
   - Evidence: lns_wizard.rs (run_devices_wizard), menu option 8 in menu_lorawan.rs
   - Gap: Formal verification not completed
   - Next: Formal TUI test needed

2. **DEV-02 (List devices with stats)**: SATISFIED
   - Evidence: LnsDeviceListRow extended with last_seen_timestamp, uplink_count
   - Query: lns_list_devices() with LEFT JOIN on sessions
   - Commit: 6414b82

3. **DEV-03 (Remove device via TUI)**: PARTIAL
   - Evidence: TOML edit + config load workflow, no direct CLI remove
   - Gap: Direct 'device remove' CLI not explicitly added
   - Workaround: wizard + file edit + reload

4. **DEV-04 (lns-config.toml import)**: SATISFIED
   - Evidence: apply_lns_config() in lns_ops.rs, menu option 6
   - Commit: 6414b82

5. **DEV-05 (Promote pending devices)**: SATISFIED
   - Evidence: lns_list_pending(), lns_approve_device(), menu option 5
   - Commit: 6414b82

6. **CORE-03 (Hardware probe on startup)**: PARTIAL
   - Evidence: probe.rs exists, visible in doctor dashboard
   - Gap: Formal verification not completed

</action>

<acceptance_criteria>
- File `.planning/phases/05-tui-device-management/05-VERIFICATION.md` exists
- Each DEV-xx requirement has status (SATISFIED/PARTIAL) with evidence
- CORE-03 has partial status documented
</acceptance_criteria>

#### Task 2: Create SUMMARY.md for Phase 05

<read_first>
- .planning/phases/05-tui-device-management/05-PLAN.md
- .planning/phases/05-tui-device-management/05-CONTEXT.md
</read_first>

<action>
Create `.planning/phases/05-tui-device-management/05-SUMMARY.md` documenting:

1. **What was built**: TUI device management with add/list/remove/approve
2. **Files modified**: LnsDeviceListRow, lns_ops.rs, menu_lorawan.rs, lns_wizard.rs
3. **Key decisions**: Wizard-based device add, SQLite-based pending device queue
4. **Tech debt**: No direct 'device remove' CLI (relies on TOML + reload)
5. **Commit reference**: 6414b82

</action>

<acceptance_criteria>
- File `.planning/phases/05-tui-device-management/05-SUMMARY.md` exists
- Documents what was built, key decisions, and tech debt
</acceptance_criteria>

#### Task 3: Verify Phase 05 implementation against plan

<read_first>
- crates/maverick-lorawan/src/lns_ops.rs
- crates/maverick-lorawan/src/lns_wizard.rs
</read_first>

<action>
Verify implementation:

1. Check lns_ops.rs contains:
   - lns_list_devices() with last_seen and uplink_count
   - lns_list_pending() and lns_approve_device()
   - apply_lns_config() for device sync

2. Check lns_wizard.rs contains run_devices_wizard

3. Check menu_lorawan.rs has:
   - Option 5: pending devices
   - Option 6: config load
   - Option 8: device wizard

4. Confirm DEV-02, DEV-04, DEV-05 are satisfied by inspection

</action>

<acceptance_criteria>
- lns_ops.rs implements list/approve functions
- wizard exists for device add
- Menu system supports required operations
</acceptance_criteria>

### must_haves

1. 05-VERIFICATION.md exists with all DEV-xx status documented
2. 05-SUMMARY.md exists documenting what was built
3. Partial requirements (CORE-03, DEV-01, DEV-03) have clear gap documentation
