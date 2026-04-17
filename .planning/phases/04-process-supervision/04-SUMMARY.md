---
phase: 4
name: Process Supervision
completed_date: 2026-04-16
commit: 89e4c62
---

# Phase 4: Process Supervision - Summary

## What Was Built

Implemented self-healing process supervision via systemd with watchdog support. Maverick now automatically recovers from crashes and hangs without operator intervention.

## Files Created/Modified

| File | Change | Purpose |
|------|--------|---------|
| `deploy/systemd/maverick-edge.service` | Created | systemd unit with Restart=always, WatchdogSec=30s |
| `crates/maverick-runtime-edge/src/watchdog.rs` | Created | sd_notify protocol implementation (READY, WATCHDOG, STOPPING) |
| `crates/maverick-runtime-edge/src/main.rs` | Modified | Integration of watchdog task in gwmp_loop |

## Key Decisions

1. **Type=notify**: Process sends sd_notify(READY=1) on startup — systemd waits for startup complete before marking service as active

2. **Watchdog interval**: 15 seconds (< WatchdogSec/2 = 15s) — ensures systemd never kills during normal operation

3. **RestartSec=2s**: Short delay before restart to allow system cleanup

4. **Security hardening**: NoNewPrivileges=true, ProtectSystem=strict, ProtectHome=true, PrivateTmp=true

5. **User/Group=maverick**: Runs as unprivileged dedicated user

## Deferred Work

### SEC-02: SQLite Key Encryption (v1.1)

**Issue**: NwkSKey and AppSKey stored as `[u8; 16]` in SessionSnapshot domain model. SQLCipher requires `Vec<u8>` for BLOB storage.

**Required Change**:
- Refactor `SessionSnapshot.nwk_s_key` and `SessionSnapshot.app_s_key` from `[u8; 16]` to `Vec<u8>`
- Update all MIC computation and payload decryption code to use `Vec<u8>`
- Implement SQLCipher key derivation and PRAGMA key injection

**Schema Comment**: Documents SQLCipher approach in `schema.rs`

**Temporary Mitigation**: SQLite file permissions (0600) restrict unprivileged access

## Verification Results

- **RELI-03**: Satisfied — systemd automatically restarts after SIGKILL
- **RELI-04**: Satisfied — watchdog prevents hung process scenarios
- **SEC-02**: Deferred — requires domain model refactor in v1.1

## Integration

- **Phase 4 → systemd**: Watchdog sends READY on startup, pings every 15s, sends STOPPING on shutdown
- **Phase 4 → Phase 1**: Depends on SQLite persistence for session storage
- **Phase 4 → Phase 5**: TUI can run independently, no hard dependency on watchdog
