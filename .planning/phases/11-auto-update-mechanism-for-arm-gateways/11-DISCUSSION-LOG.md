# Phase 11: Auto-update mechanism for ARM gateways — Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-04-17
**Phase:** 11-auto-update-mechanism-for-arm-gateways
**Areas discussed:** Update Source Strategy, Update Trigger Mechanism, Atomic Update Pattern, Version Checking, Update Artifact Handling, Permissions and Security, Integration with Phase 4 (systemd supervision), CLI Commands (Operator Interface), Build/Cross-Compilation for ARM

---

## [auto] All Gray Areas

**[auto] Selected all gray areas:** Update Source Strategy, Update Trigger Mechanism, Atomic Update Pattern, Version Checking, Update Artifact Handling, Permissions and Security, Integration with Phase 4 (systemd supervision), CLI Commands (Operator Interface), Build/Cross-Compilation for ARM

**[auto] Update Source Strategy — Q: "How should the device get new updates?" → Selected: "Dev (git pull + build) + Prod (release download)" (recommended default)**
**Rationale:** Dual-mode allows both development self-hosted scenarios (git pull) and production (pre-built releases). Dev mode is opt-in via config. Production is the default.

**[auto] Update Trigger Mechanism — Q: "What triggers the update check?" → Selected: "systemd timer" (recommended default)**
**Rationale:** Script-based update + systemd timer is simple and reliable. No background thread, no daemon overhead. Timer fires every check_interval, script runs, exits.

**[auto] Atomic Update Pattern — Q: "How should the update be applied atomically?" → Selected: "Stop service, replace binary, restart (systemd handles restart)" (recommended default)**
**Rationale:** The update script stops the service, replaces the binary atomically (copy to .new, mv), then restarts. systemd Restart=always handles crash recovery. No automatic rollback — operator intervenes if broken.

**[auto] Version Checking — Q: "How should version comparison work?" → Selected: "version.txt for prod, git describe --tags for dev" (recommended default)**
**Rationale:** Prod mode uses a simple version.txt file alongside the release. Dev mode uses git describe --tags. Both are lightweight and work offline.

**[auto] Update Artifact Handling — Q: "Where should downloaded binaries and backups be stored?" → Selected: "/var/lib/maverick/downloads/ and /var/lib/maverick/backups/" (recommended default)**
**Rationale:** Standard directories under /var/lib/maverick/. Backup retention: last 2 backups. Operator can set download_dir to tmpfs for storage-constrained devices.

**[auto] Permissions and Security — Q: "What permissions and security measures?" → Selected: "root execution, 0755 binary, HTTPS for prod, journald logging" (recommended default)**
**Rationale:** Update script runs as root (required to write binary and restart service). HTTPS verification for prod downloads. Logs to journald for audit trail.

**[auto] Integration with Phase 4 (systemd supervision) — Q: "How does the update mechanism integrate with existing systemd supervision?" → Selected: "Separate service + timer, orthogonal to existing service" (recommended default)**
**Rationale:** maverick-update.service (Type=oneshot) + maverick-update.timer. No changes to maverick-edge.service. Updates are independent of the supervised runtime.

**[auto] CLI Commands (Operator Interface) — Q: "What CLI commands should operators have?" → Selected: "update check, update status, update history" (recommended default)**
**Rationale:** Three commands: check (manual trigger), status (current version, last update info), history (recent attempts from journal). Standard update operator interface.

**[auto] Build/Cross-Compilation for ARM — Q: "How should ARM binaries be built and distributed?" → Selected: "x86_64 CI builds ARM binaries, uploads to release URL" (recommended default)**
**Rationale:** Cross-compilation via zig or cross on x86_64 CI. Binary naming: maverick-edge-{arch}-{version}. Device downloads correct binary based on its arch detected at runtime.

---

## Claude's Discretion

- All gray areas selected with recommended defaults (noted in each decision above)
- No further clarification needed — implementation can proceed from CONTEXT.md decisions

## Deferred Ideas

- Cloud-initiated updates (push from server) — v2 feature
- Delta updates (binary diffs) — v2
- Automatic rollback to previous backup — v2
- Update to extensions (TUI, etc.) — v2
- Multi-binary component updates — v2