---
phase: "06"
name: "Phase 4 Verification Gap Closure"
wave: 1
depends_on: []
autonomous: true
requirements_addressed:
  - RELI-03
  - RELI-04
  - SEC-02
files_modified: []
---

## Plan: Phase 4 Verification Gap Closure

### Objective

Create formal VERIFICATION.md and SUMMARY.md for Phase 4 (Process Supervision) to address audit gaps. SEC-02 (SQLite key encryption) remains deferred as it requires SessionSnapshot domain model changes.

### Success Criteria

1. **VERIFICATION.md exists** in .planning/phases/04-process-supervision/ with test evidence for RELI-03, RELI-04
2. **SUMMARY.md exists** in .planning/phases/04-process-supervision/ documenting what was built
3. **SEC-02 gap documented** as deferred with clear blocking issue

### Tasks

#### Task 1: Create VERIFICATION.md for Phase 04

<read_first>
- .planning/v1.0-MILESTONE-AUDIT.md (audit findings)
- .planning/phases/04-process-supervision/04-PLAN.md (original plan)
- deploy/systemd/maverick-edge.service (if exists)
</read_first>

<action>
Create `.planning/phases/04-process-supervision/04-VERIFICATION.md` documenting:

1. **RELI-03 Verification**: After SIGKILL, maverick-edge automatically restarted by systemd within 2 seconds
   - Evidence: systemd service Type=notify, Restart=always, RestartSec=2s
   - Test: `kill -9 <pid>` and verify process restarts

2. **RELI-04 Verification**: Hung process detected by systemd WatchdogSec
   - Evidence: watchdog.rs pings every 15s, WatchdogSec=30s in service file
   - Test: Block watchdog pings and verify systemd kills process

3. **SEC-02 Status**: DEFERRED
   - Blocking: SessionSnapshot domain model uses [u8;16] not Vec<u8>
   - Schema comment documents approach
   - Target: v1.1

</action>

<acceptance_criteria>
- File `.planning/phases/04-process-supervision/04-VERIFICATION.md` exists
- Contains verification evidence for RELI-03 and RELI-04
- Contains DEFERRED status for SEC-02 with blocking issue documented
</acceptance_criteria>

#### Task 2: Create SUMMARY.md for Phase 04

<read_first>
- .planning/phases/04-process-supervision/04-PLAN.md
- .planning/phases/04-process-supervision/04-CONTEXT.md
</read_first>

<action>
Create `.planning/phases/04-process-supervision/04-SUMMARY.md` documenting:

1. **What was built**: systemd service with watchdog, Type=notify
2. **Files created/modified**: maverick-edge.service, watchdog.rs
3. **Key decisions**: watchdog pings every 15s, WatchdogSec=30s, RestartSec=2s
4. **Deferred work**: SEC-02 SQLite encryption (blocking: domain model)
5. **Commit reference**: 89e4c62

</action>

<acceptance_criteria>
- File `.planning/phases/04-process-supervision/04-SUMMARY.md` exists
- Documents what was built, key decisions, and deferred work
</acceptance_criteria>

#### Task 3: Verify Phase 04 implementation against plan

<read_first>
- deploy/systemd/maverick-edge.service
- crates/maverick-runtime-edge/src/watchdog.rs
</read_first>

<action>
Verify the implementation matches the plan:

1. Check `maverick-edge.service` contains:
   - Type=notify
   - Restart=always
   - RestartSec=2s
   - WatchdogSec=30s

2. Check `watchdog.rs`:
   - Spawns watchdog task in gwmp_loop
   - Sends WATCHDOG=1 notifications every 15s

3. Confirm RELI-03 and RELI-04 are satisfied

</action>

<acceptance_criteria>
- systemd service file has correct configuration
- watchdog.rs implements watchdog ping loop
- RELI-03 and RELI-04 verified by inspection
</acceptance_criteria>

### must_haves

1. 04-VERIFICATION.md exists with evidence for RELI-03, RELI-04
2. 04-SUMMARY.md exists documenting what was built
3. SEC-02 deferred status clearly documented
