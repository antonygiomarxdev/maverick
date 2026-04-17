# Phase 4: Process Supervision - Context

**Gathered:** 2026-04-16
**Status:** Ready for planning
**Mode:** Auto-generated (discuss skipped via autonomous workflow)

<domain>
## Phase Boundary

Maverick-edge process self-heals after any crash or hang via systemd supervision. Watchdog detects hung processes. Session keys (NwkSKey, AppSKey) are protected with SQLite encryption so unprivileged users cannot read them as plaintext.

</domain>

<decisions>
## Implementation Decisions

### Claude's Discretion
All implementation choices are at Claude's discretion. Use ROADMAP phase goal, success criteria, and codebase conventions to guide decisions.

### Key Technical Decisions Needed
- **systemd unit file**: Create maverick-edge.service with Restart=always, WatchdogSec
- **Watchdog implementation**: sd_notify protocol for watchdog pings
- **SQLite encryption**: Use SQLCipher or rusqlite_bundled with SQLITE_HAS_CODEC

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/maverick-runtime-edge/src/main.rs`: Entry point where supervision should be integrated
- `crates/maverick-adapter-persistence-sqlite/src/persistence/repos.rs`: Session storage with keys

### Established Patterns
- systemd service files follow Debian packaging conventions
- Watchdog uses sd_notify(3) protocol via libc

### Integration Points
- Process startup/shutdown hooks in main.rs
- SQLite connection initialization for encryption key

</code_context>

<specifics>
## Specific Ideas

Requirements: RELI-03, RELI-04, SEC-02

</specifics>

<deferred>
## Deferred Ideas

None.

</deferred>
