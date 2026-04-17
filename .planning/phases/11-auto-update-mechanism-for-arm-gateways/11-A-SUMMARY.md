---
phase: 11-auto-update-mechanism-for-arm-gateways
plan: A
subsystem: update
tags: [systemd, bash, arm, update]

requires: []
provides:
  - systemd timer (maverick-update.timer) for hourly update checks
  - systemd oneshot service (maverick-update.service) for atomic updates
  - Bash update script with release and dev mode support
  - Default configuration in /etc/maverick/maverick.toml
affects: [phase-11-B, phase-11-C]

tech-stack:
  added: [systemd, bash, systemd-cat]
  patterns: [atomic-update, journald-logging]

key-files:
  created:
    - bin/maverick-update.sh
    - etc/maverick-update.service
    - etc/maverick-update.timer
    - etc/maverick.toml
  modified: []

key-decisions:
  - "Atomic update pattern: copy to .new suffix, then rename to avoid partial writes"
  - "Type=oneshot service exits after completion, timer handles scheduling"
  - "Two update modes: release (download binary) and dev (git pull + cargo build)"

patterns-established:
  - "Systemd timer + oneshot service pattern for scheduled updates"
  - "Journald logging via systemd-cat for audit trail"

requirements-completed: []

duration: 5min
completed: 2026-04-17
---

# Phase 11 Plan A: Auto-update Shell Script and Systemd Files

**Systemd timer + bash script for atomic self-updates on ARM gateways**

## Performance

- **Duration:** 5 min
- **Started:** 2026-04-17T00:00:00Z
- **Completed:** 2026-04-17T00:00:05Z
- **Tasks:** 1 (shell script and systemd files)
- **Files modified:** 4 created

## Accomplishments
- Created `/usr/local/bin/maverick-update.sh` executable update script
- Created `/etc/systemd/system/maverick-update.service` (Type=oneshot)
- Created `/etc/systemd/system/maverick-update.timer` (fires hourly after 5min boot delay)
- Created `/etc/maverick/maverick.toml` with default update configuration

## Task Commits

Each task was committed atomically:

1. **Task 1: Create update script and systemd files** - `56acd89` (feat)

**Plan metadata:** `56acd89` (docs: complete plan)

## Files Created/Modified
- `bin/maverick-update.sh` - Atomic update script with release/dev mode
- `etc/maverick-update.service` - systemd oneshot service unit
- `etc/maverick-update.timer` - systemd timer unit (hourly + 5min boot)
- `etc/maverick.toml` - Default update configuration

## Decisions Made
- Used Type=oneshot for service that exits after update check
- Timer fires every hour (OnUnitActiveSec=1h) with 5 minute initial delay
- Journald logging via systemd-cat -t maverick-update for audit trail
- Atomic replace: copy to .new suffix, then rename to avoid partial writes

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None

## Next Phase Readiness
- Update script foundation ready for Plan B (Rust update module)
- CLI integration in Plan C will call into update module

---
*Phase: 11-auto-update-mechanism-for-arm-gateways*
*Completed: 2026-04-17*
