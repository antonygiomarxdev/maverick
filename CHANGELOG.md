# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog.

## [Unreleased]

### Changed

- Documentation and governance hardening for open-source release readiness:
  - added extension version-lock and compatibility policy,
  - added per-extension READMEs,
  - aligned release/install/runbook wording with artifact/tag behavior,
  - replaced placeholder maintainer/security metadata.

## [0.1.0] - 2026-04-10

### Added

- Hexagonal core structure in `maverick-core`:
	- `ports`, `use_cases`, `adapters`, `kernel`, `events`, `ingester`, `config` modules.
	- Device management use cases and HTTP boundary DTOs.
	- Downlink enqueue/status and delivery worker loop with retry/fail transitions.
	- Semtech UDP ingestion path and LoRaWAN frame processing base flow.
	- Storage profile model (`auto`, `high`, `mid`, `extreme`) with hardware-aware guard.
	- SQLite schema bootstrap and persistence adapters for device/gateway/session/uplink/downlink/audit.
- Runtime operational features:
	- Circular uplink buffer for extreme profile.
	- Batch writer for uplink persistence.
	- Retention service for expirable records.
- Test coverage additions:
	- API/integration tests for health, device CRUD, downlinks, and UDP ingest.
	- Unit tests for sqlite adapters, storage profile behavior, retention, and delivery service.
- Open source governance baseline:
	- `LICENSE`, `CODE_OF_CONDUCT.md`, `CONTRIBUTING.md`, `SECURITY.md`, `.github` templates, `CODEOWNERS`.
- Deployment and release assets:
	- `Dockerfile`, `docker-compose.yml`, profile env presets (`edge`, `gateway`, `server`).
	- Installer script and profile runner script.
	- Release workflow for multi-target binary artifacts and multi-arch container image publishing.
- Repository hygiene/tooling:
	- `.editorconfig`, `.gitattributes`, `rustfmt.toml`, `.clippy.toml`.
	- `Cargo.lock` now tracked for reproducible builds.

### Changed

- README rewritten to reflect current implemented capabilities, quickstart usage, deployment profiles, and roadmap.
- CI workflow expanded with lint/test separation and dependency audit job.

### Fixed

- Line ending normalization to LF across tracked text files for cross-platform consistency.
