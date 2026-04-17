# Phase 11: Auto-update mechanism for ARM gateways — Context

**Gathered:** 2026-04-17
**Status:** Ready for planning

<domain>
## Phase Boundary

Create a self-contained auto-update mechanism that allows Maverick to update itself on ARM devices (like RAK Pi). The update must be atomic — stop service, replace binary, restart — and resilient to partial failures. Supports both development (git pull + build) and production (release download from URL) paths.

**Out of scope:** Cloud-initiated updates, delta updates, rollback automation beyond restart, multi-binary component updates

</domain>

<decisions>
## Implementation Decisions

### Update Source Strategy

- **D-01:** Support two update modes:
  - **Dev mode:** `git pull + build` — builds from source on the device
  - **Prod mode:** Download pre-built release binary from a URL
- **D-02:** Mode selection via config: `update.mode = "dev" | "release"` (default: `release`)
- **D-03:** Dev mode requires `git` and Rust toolchain on device — operator opts in knowingly
- **D-04:** Prod mode downloads from `update.release_url` — URL can point to GitHub releases or private server

### Update Trigger Mechanism

- **D-05:** Update check triggered by a **systemd timer** (not inotify or background thread)
- **D-06:** Timer fires every `update.check_interval` (default: `3600` seconds = 1 hour)
- **D-07:** On trigger: update script runs, checks version, fetches/builds if newer, replaces binary, exits
- **D-08:** **No automatic restart** — update script exits after binary replacement; systemdRestart=always handles restart
- **D-09:** Separate systemd service (`maverick-update.service`) + timer (`maverick-update.timer`) — not a long-running process

### Atomic Update Pattern

- **D-10:** Update script stops `maverick-edge.service`, replaces binary, starts service — all in one script
- **D-11:** Binary replaced atomically: copy new binary to `.new` suffix, `mv .new binary`, then restart
- **D-12:** If service fails to start after update, systemd Restart=always kicks in (previous working binary used until operator intervenes)
- **D-13:** **No automatic rollback** — if update breaks binary, service repeatedly restarts; operator manually fixes

### Version Checking

- **D-14:** Version comparison: local `MAVERICK_VERSION` env var vs remote version file or git tag
- **D-15:** For prod: a `version.txt` file hosted alongside the release binary (simple `MAJOR.MINOR.PATCH` string)
- **D-16:** For dev: `git describe --tags` compared against last successful update's tracked ref
- **D-17:** Version check must handle network failures gracefully — log and exit without updating

### Update Artifact Handling

- **D-18:** Downloaded binary saved to `update.download_dir` (default: `/var/lib/maverick/downloads/`)
- **D-19:** Old binary backed up to `update.backup_dir` (default: `/var/lib/maverick/backups/`) with timestamp suffix
- **D-20:** Backup retention: keep last 2 backups (automatic cleanup of older backups)
- **D-21:** Storage-constrained devices (SD card): operator can set `update.download_dir` to tmpfs mount

### Permissions and Security

- **D-22:** Update script runs as root (required to write to binary path and restart service)
- **D-23:** Binary ownership: root:root, mode 0755 (executable)
- **D-24:** Prod mode: verify HTTPS certificate for download URL (no insecure http unless `update.insecure = true`)
- **D-25:** Update script logs to journald (`systemd-cat -t maverick-update`) for audit trail

### Integration with Phase 4 (systemd supervision)

- **D-26:** Update mechanism is orthogonal to existing `maverick-edge.service` (no changes to existing service)
- **D-27:** `maverick-update.service` is a `Type=oneshot` unit that exits after update attempt
- **D-28:** `maverick-update.timer` is a `AccuracySec=1min` timer that fires every `update.check_interval`

### CLI Commands (Operator Interface)

- **D-29:** `maverick-edge update check` — run update check manually, log result, exit
- **D-30:** `maverick-edge update status` — show current version, last update time, last update result
- **D-31:** `maverick-edge update history` — show last N update attempts from journal

### Build/Cross-Compilation for ARM

- **D-32:** Prod releases for ARM: x86_64 CI builds aarch64/armv7 binaries and uploads to release URL
- **D-33:** Binary naming convention: `maverick-edge-{arch}-{version}` (e.g., `maverick-edge-aarch64-1.1.0`)
- **D-34:** Build machine must have correct cross-compilation toolchain (zig or cross) — handled in CI, not on device

### Prior Decisions (locked from earlier phases)

- **Phase 4:** `maverick-edge.service` uses `Restart=always` and `WatchdogSec=30`
- **Phase 4:** Binary installed at `/usr/local/bin/maverick-edge`
- **Phase 4:** Service runs as `maverick` user (not root) — update script needs root for binary replacement
- **Phase 2/10:** `maverick-adapter-radio-spi` compiled as separate archive when `spi` feature active
- **Phase 5:** `maverick-edge` is the single binary for all functionality (TUI, radio, runtime)

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Prior phase context
- `.planning/phases/10-libloragw-spi-integration/10-CONTEXT.md` — Phase 10 SPI integration context
- `.planning/phases/04-process-supervision/04-CONTEXT.md` — systemd service file structure
- `.planning/phases/05-tui-device-management/05-CONTEXT.md` — CLI command patterns

### Architecture decisions
- `.planning/PROJECT.md` — offline-first, self-contained, no external service calls
- `.planning/PROJECT.md` §Constraints — Linux-only, armv7 with ≤512 MB RAM, SQLite persistence
- `.planning/PROJECT.md` §Principles — "Reliability above all" — the LNS core never falls

### Related (do not implement)
- `.planning/ROADMAP.md` §Phase 11 — placeholder, no details yet
- `.planning/STATE.md` — current state (v1.0 shipped, Phase 11 next)
- `docs/code-review-checklist.md` — checklist to follow before PR/merge

</canonical_refs>

<code_context>
## Existing Code Insights

### Update-worthy binary location
- Binary installed at `/usr/local/bin/maverick-edge` (from Phase 4 systemd service)

### Systemd patterns (from Phase 4)
- `maverick-edge.service` — `Type=simple`, `Restart=always`, `WatchdogSec=30`, runs as `maverick` user
- Service file likely in `/etc/systemd/system/maverick-edge.service`
- Logs via journald (`journalctl -u maverick-edge`)

### CLI command patterns (from Phase 5)
- `maverick-edge device list` — shows devices, last-seen, uplink count
- `maverick-edge hardware probe` — shows CPU, RAM, storage, arch
- Commands are structured as subcommands (update, device, hardware, etc.)

### Phase 10 SPI libloragw integration (just completed)
- Binary is built with `CARGO_FEATURE_SPI` for ARM targets
- `build.rs` compiles vendored C when feature active
- Cross-compilation happens on x86_64 build machine

</code_context>

<specifics>
## Specific Ideas

- Update script: Bash script at `/usr/local/bin/maverick-update.sh` — stop service, replace binary, start service, log
- systemd service: `/etc/systemd/system/maverick-update.service` — runs update.sh as root
- systemd timer: `/etc/systemd/system/maverick-update.timer` — fires every check_interval
- Config in `maverick.toml`:
  ```toml
  [update]
  mode = "release"           # "release" or "dev"
  release_url = "https://github.com/yourorg/maverick/releases"
  check_interval = 3600       # seconds
  download_dir = "/var/lib/maverick/downloads"
  backup_dir = "/var/lib/maverick/backups"
  ```
- Version file: `https://releases.example.com/maverick/aarch64/version.txt` contains `1.1.0`
- Binary URL: `https://releases.example.com/maverick/aarch64/maverick-edge-1.1.0`
- Dev mode: `git clone`, `cargo build --release`, replace binary

</specifics>

<deferred>
## Deferred Ideas

- Cloud-initiated updates (push from server) — v2 feature, requires cloud connection
- Delta updates (binary diffs) — v2, saves bandwidth but adds complexity
- Automatic rollback to previous backup — v2, needs health-check before restart
- Update to extensions (TUI, etc.) — v2, extensions are separate processes
- Multi-binary component updates — v2, currently only one binary

---

*Phase: 11-auto-update-mechanism-for-arm-gateways*
*Context gathered: 2026-04-17*
*Auto-discuss: All gray areas selected with recommended defaults*