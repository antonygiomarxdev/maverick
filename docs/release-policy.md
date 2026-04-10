# Release and Versioning Policy

Date: 2026-04-10
Status: Active

## Goal

Provide predictable, reproducible releases for operators and contributors.

## When GitHub Actions run

| Workflow | File | When it runs |
|----------|------|----------------|
| **CI** | `.github/workflows/ci.yml` | Every push to `main` and every pull request. |
| **Release** | `.github/workflows/release.yml` | **Only** when a version tag matching `v*` is **pushed**, or when a maintainer runs **Release** manually (**Actions** → **Release** → **Run workflow**) and enters a `version_tag` (e.g. `v0.1.0`). |

Pushing commits to `main` without a tag **does not** build release tarballs or create a GitHub Release. That is why `install-linux.sh --version latest` returns 404 until the first successful Release workflow completes.

### First release (pick one)

1. **Tag push (typical):** after `main` has what you want to ship, on your machine:
   - `git tag v0.1.0`
   - `git push origin v0.1.0`
2. **Manual run:** GitHub → **Actions** → **Release** → **Run workflow**, set **version tag** to `v0.1.0`, run on `main` (or the branch you intend to ship).

Then open **Actions** and confirm the **Release** workflow is green; after that, **Releases** should list assets and `/releases/latest` will work for the installer.

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
8. **Cross targets (aarch64 / armv7):** run the Docker smoke build so `libsqlite3-sys` does not surprise you on GitHub runners (see below).

## Validate release cross-target builds locally

The Release workflow cross-compiles `maverick-runtime-edge` and `maverick-extension-tui` for **aarch64** and **armv7**. Host headers (`/usr/include`) must not be mixed with the cross compiler: CI sets `CFLAGS_*=--sysroot=...` for that reason.

**Before pushing a tag**, from the repository root (requires [Docker](https://docs.docker.com/get-docker/): Linux, WSL2, or Docker Desktop):

```bash
bash scripts/verify-release-cross-builds.sh
```

Optional: `RUST_VERIFY_IMAGE=rust:1-bookworm` (default) or another `rust:*-bookworm` image if you need a specific toolchain.

This runs the same Debian packages and environment variables as `.github/workflows/release.yml` for those targets, plus a quick native `x86_64` build inside the container.

## Post-release checks

After workflow completion:

1. Confirm GitHub Release includes all expected tarballs and checksum files.
2. Verify checksum validation works for at least one target.
3. Confirm GHCR received expected tags (`vX.Y.Z`, `X.Y`, and `latest` only for stable).
