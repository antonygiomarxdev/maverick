# Extensions and Versioning Policy

Date: 2026-04-10
Status: Active

## Purpose

This document defines how Maverick extensions are versioned, released, and validated against the core runtime.

## Extension model

- Core runtime binary: `maverick-edge`
- Optional extension binaries: for example `maverick-edge-tui`
- Shared contracts crate: `maverick-extension-contracts`

Extensions are additive. The default operator path remains the core CLI.

## Version lock policy (v1.x)

Maverick uses a workspace-wide release cadence for v1.x:

- A single git tag (for example `v0.1.0`) represents one coherent release across core and extensions.
- Workspace package version in `Cargo.toml` is the source of truth.
- Core and extension crates in this monorepo are released and documented together.

Practical rule:

- `maverick-edge` and `maverick-edge-tui` should run the same release version tag in production.

## Compatibility contract

- `maverick-extension-contracts` carries the stable contract surface for extension/core boundaries.
- Patch releases (`x.y.z`) must be backward-compatible.
- Minor releases (`x.y.0`) may add fields/commands in a backward-compatible way.
- Major releases (`x.0.0`) may include breaking changes and require explicit migration notes.

## Release artifacts

The release workflow publishes:

- Linux tarballs per target with both binaries (`maverick-edge` and `maverick-edge-tui`)
- SHA256 files for each tarball
- GHCR container image tags for runtime deployment

Operators should install from tagged releases and verify checksums before use.

## Deferred issue triage (Phase 2)

Issue cleanup and ongoing visibility are handled with milestones and status labels.

### Issue taxonomy

- Status labels:
  - `status:planned`
  - `status:in-progress`
  - `status:blocked`
  - `status:done`
- Domain labels stay in use (`kernel`, `ops`, `resilience`, `observability`, etc.).

### Milestone policy

- Use version milestones for committed work (`v0.1.0`, `v0.1.1`, ...).
- Use `backlog` milestone for non-committed or later-scope items.

### Triage policy

- Close completed issues with evidence links (tests/docs/PR references).
- Close duplicates with canonical issue references.
- Apply stale closure only after explicit warning and inactivity window.
- Move support-like discussions away from bug tracking when a dedicated support channel exists.
