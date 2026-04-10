# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog.

## [Unreleased]

Nothing yet.

## [0.1.0] - 2026-04-10

First **public beta** release: Linux binaries, installer, and documented release process. Version `0.1.0` matches `[workspace.package] version` in `Cargo.toml`; tag **`v0.1.0`**.

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
	- Release workflow for multi-target Linux tarballs (with SHA256), multi-arch GHCR image, optional `workflow_dispatch`.
- Optional terminal extension: `maverick-extension-tui` (`maverick-edge-tui`) shipped in the same release tarball as `maverick-edge`.
- `scripts/install-linux.sh` with one-liner install (`curl ... | bash -s --`), architecture auto-detect, checksum verify, and optional TUI install when present in the archive.
- `docs/install.md`, `docs/release-policy.md`, `docs/extensions.md`; per-extension READMEs; operator GWMP local-gateway integration test.
- Repository hygiene/tooling:
	- `.editorconfig`, `.gitattributes`, `rustfmt.toml`, `.clippy.toml`.
	- `Cargo.lock` now tracked for reproducible builds.

### Changed

- Declared **public beta** policy: `0.x` line is the beta train; `1.0.0` marks intentional stable graduation (see `docs/release-policy.md`, `README.md`).
- README as OSS landing page (badges, install, releases, contributing); docs index updated.
- Open-source metadata: workspace `repository` and `authors`; SECURITY private advisories; CoC reporting path; CONTRIBUTING aligned with CI commands and issue hygiene; issue templates + `config.yml`; PR template verification commands.
- Release/install alignment: extension version-lock docs; documented **CI** (every `main` push) vs **Release** (tag `v*` or manual dispatch); GHCR `latest` only for stable tags (no prerelease suffix in tag name).
- Install UX: explicit error when no GitHub Release exists yet; fixed help text when the script is run via `bash -s` from a pipe.

### Fixed

- Line ending normalization to LF across tracked text files for cross-platform consistency.
