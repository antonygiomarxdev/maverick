# Release and Versioning Policy

Date: 2026-04-10
Status: Active

## Goal

Provide predictable, reproducible releases for operators and contributors.

## Project phase: public beta

Maverick is in **public beta**. Shipping is driven by **annotated git tags** `v0.x.y`:

- The **`0.x` line is the beta train**: APIs, CLI flags, and defaults may evolve.
- **`1.0.0` is reserved** for the first intentionally stable major once the team graduates beta.
- **Every meaningful operator-facing change** should land in a tagged release with `CHANGELOG.md` updated, so installs can pin a version.

`main` may move ahead of the latest tag; operators should prefer **Releases** unless they build from source.

## Version scheme

Maverick follows semantic versioning with git tags in the form `vX.Y.Z`.

- Patch (`0.y.z` while in beta): fixes and behavior-preserving hardening.
- Minor (`0.y.0` while in beta): new features and operational improvements (may include breaking changes while major is 0).
- Major (`1.0.0` and onward): breaking changes require migration guidance; after 1.0, follow strict SemVer for public contracts.

## What is released

For each release tag:

- GitHub Release with generated release notes.
- Linux tarballs per target containing:
  - `maverick-edge`
  - `maverick-edge-tui`
  - `install-linux.sh`
- SHA256 checksum file for each tarball.
- GHCR container image tags.

## Docker tag policy

Published tags:

- `ghcr.io/antonygiomarxdev/maverick:vX.Y.Z`
- `ghcr.io/antonygiomarxdev/maverick:X.Y`
- `ghcr.io/antonygiomarxdev/maverick:latest` only for stable tags (no prerelease suffix).

If a tag contains a prerelease suffix (for example `v0.2.0-rc.1`), `latest` must not move.

## Version lock (core + official extensions)

For every tagged release:

- `maverick-edge` and `maverick-edge-tui` should run the same release tag.
- Workspace package version in `Cargo.toml` is the source of truth and must match the tag (without the `v` prefix).

## Pre-release checklist

Before creating a tag:

1. Bump `[workspace.package] version` in `Cargo.toml` to match the upcoming tag (for example tag `v0.2.0` implies version `0.2.0`).
2. `cargo fmt --all -- --check`
3. `cargo clippy --workspace --all-targets --all-features -- -D warnings`
4. `cargo test --workspace`
5. Commit `Cargo.lock` if dependency resolution changed.
6. Move relevant entries from `Unreleased` in `CHANGELOG.md` into the target version section.
7. Verify install docs and runbook changes when operator behavior changed.

## Post-release checks

After workflow completion:

1. Confirm GitHub Release includes all expected tarballs and checksum files.
2. Verify checksum validation works for at least one target.
3. Confirm GHCR received expected tags (`vX.Y.Z`, `X.Y`, and `latest` only for stable).
