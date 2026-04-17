---
phase: 4
name: Process Supervision
verification_date: 2026-04-16
status: complete_with_deferred
---

# Phase 4: Process Supervision - Verification

## Requirements Verification

### RELI-03: systemd Restart=always

**Status**: SATISFIED

**Verification Evidence**:
- systemd service file: `deploy/systemd/maverick-edge.service`
- Configuration:
  - `Type=notify` — process sends sd_notify(READY=1) on startup
  - `Restart=always` — systemd restarts on any exit
  - `RestartSec=2s` — restart delay
  - `User=maverick` / `Group=maverick` — runs as unprivileged user

**Test Method**:
1. Install: `sudo cp deploy/systemd/maverick-edge.service /etc/systemd/system/`
2. Start: `sudo systemctl start maverick-edge`
3. Verify running: `sudo systemctl status maverick-edge`
4. Simulate crash: `sudo kill -9 $(pidof maverick-edge)`
5. Verify restart: `sudo systemctl status maverick-edge` (should show "running" within 2 seconds)

**Commit Reference**: 89e4c62

---

### RELI-04: WatchdogSec hung process detection

**Status**: SATISFIED

**Verification Evidence**:
- `WatchdogSec=30s` in service file
- `watchdog.rs` sends WATCHDOG=1 ping every 15 seconds (interval < WatchdogSec/2)
- sd_notify protocol implementation:
  - `send_ready()` — called on startup (READY=1)
  - `send_watchdog_ping()` — called every 15s in watchdog task
  - `send_stopping()` — called on graceful shutdown (STOPPING=1)

**Test Method**:
1. Block watchdog pings: `systemd-run --scope -p WatchdogSec=1s /bin/sleep 60`
2. Observe: systemd should kill process after 30s without watchdog ping
3. Verify: `journalctl -u maverick-edge` shows process killed and restarted

**Commit Reference**: 89e4c62

---

### SEC-02: SQLite key encryption

**Status**: DEFERRED

**Deferred Reason**: Full SQLCipher encryption requires SessionSnapshot domain model refactor. Current implementation typed as `[u8;16]` not `Vec<u8>`, blocking SQLCipher key integration.

**Evidence of Attempt**:
- Schema comment documents SQLCipher approach in `schema.rs`
- Plan created with SQLCipher implementation details
- Domain model changes required before implementation

**Blocking Issue**:
```
SessionSnapshot.nwk_s_key and SessionSnapshot.app_s_key
currently typed as: [u8; 16]
needs to be: Vec<u8> for SQLCipher BLOB compatibility
```

**Target**: v1.1 (requires Phase 1 domain model refactor)

**Workaround**: SQLite file permissions (0600) restrict access to unprivileged users, but this does not satisfy the requirement that keys not be readable as plaintext.

---

## Summary

| Requirement | Status | Evidence |
|-------------|--------|----------|
| RELI-03 | SATISFIED | Type=notify, Restart=always, RestartSec=2s |
| RELI-04 | SATISFIED | WatchdogSec=30s, watchdog ping every 15s |
| SEC-02 | DEFERRED | Domain model [u8;16] → Vec<u8> refactor needed |

**Phase Status**: COMPLETE (with SEC-02 deferred to v1.1)
