# Phase 12: Release CI hardening and update URL configuration — Context

**Gathered:** 2026-04-17
**Status:** Ready for planning

<domain>
## Phase Boundary

Release infrastructure hardening for Maverick v1.x releases:
1. Update `release_url` in `maverick.toml` to point to GitHub Releases
2. Add SBOM (Software Bill of Materials) generation to CI
3. Add code signing to CI for binary authenticity
4. Create `SECURITY.md` vulnerability disclosure policy
5. Create `version.txt` artifact in releases for auto-update mechanism

**Out of scope:** Cloud-initiated updates, multi-binary updates, delta updates

</domain>

<decisions>
## Implementation Decisions

### URL Configuration

- **D-01:** `release_url` in `maverick.toml` set to `https://github.com/antonygiomarxdev/maverick/releases/download`
- **D-02:** Auto-update mechanism (from Phase 11) expects `version.txt` at release URL root

### SBOM Generation

- **D-03:** Generate SBOM using **Syft** (industry standard, GitHub-native integration)
- **D-04:** SBOM format: **SPDX JSON** (machine-readable, widely supported)
- **D-05:** SBOM uploaded as release artifact alongside binaries
- **D-06:** SBOM included in `release.yml` `publish-github-release` job

### Code Signing

- **D-07:** Use **SIGSTORE** (`sigstore/github-actions`) for keyless code signing
- **D-08:** Sign each binary tarball and SHA256 checksum file
- **D-09:** Signature files (`.sig`) uploaded as release artifacts
- **D-10:** Verification instructions documented in release notes

### Security Policy

- **D-11:** Create `SECURITY.md` at repository root
- **D-12:** Use **GitHub Security Advisories** for vulnerability reporting (GHSA)
- **D-13:** Disclosure policy: **Coordinated disclosure** — reporter notifies, maintainer responds within 7 days, fix timeline communicated
- **D-14:** Scope: All LoRaWAN protocol handling, MIC verification, session key handling
- **D-15:** Contact: GitHub's private vulnerability reporting via `Security` tab

### Version Artifact

- **D-16:** Create `version.txt` file containing the version string (e.g., `1.0.0`)
- **D-17:** `version.txt` included in each release artifact tarball
- **D-18:** Auto-update script (Phase 11) reads `version.txt` to check for newer version

### Prior Decisions (locked from earlier phases)

- **Phase 11:** Auto-update checks `release_url` for `version.txt` to compare versions
- **Phase 11:** Binary naming: `maverick-{arch}-{version}` format
- **Phase 4:** Binary installed at `/usr/local/bin/maverick-edge`
- **Phase 4:** Service runs as `maverick` user, update script runs as root

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Prior phase context
- `.planning/phases/11-auto-update-mechanism-for-arm-gateways/11-CONTEXT.md` — Phase 11 auto-update mechanism
- `.planning/PROJECT.md` — offline-first, self-contained, no external service calls
- `.planning/ROADMAP.md` §Phase 12 — Phase 12 placeholder

### CI/CD references
- `.github/workflows/release.yml` — existing release workflow to extend

### Security references
- [Sigstore GitHub Actions](https://github.com/sigstore/sigstore-github-actions) — keyless signing
- [Syft GitHub Actions](https://github.com/anchore/sbom-action) — SBOM generation
- [GitHub Security Advisories](https://docs.github.com/en/security/security-advisories/guided-onboarding/creating-a-security-advisory) — vulnerability reporting

</canonical_refs>

<codebase_context>
## Existing Code Insights

### Release workflow structure
- `.github/workflows/release.yml` — existing workflow with `prepare`, `build`, `publish-github-release` jobs
- Multi-arch builds: x86_64, aarch64, armv7
- Artifacts: `*.tar.gz` + `*.sha256` files

### Update mechanism (Phase 11)
- `maverick.toml` has `[update]` section with `release_url`, `check_interval`, etc.
- `version.txt` expected at release URL root for version comparison

</codebase_context>

<specifics>
## Specific Ideas

- SBOM action: `anchore/sbom-action@v0` with Syft analyzer
- Sigstore signing: `sigstore/sigstore-github-actions/sign-blobs@main` with `private-key` input
- SECURITY.md template from GitHub's `SECURITY.md` spec with custom disclosure policy
- version.txt created in `build` job, embedded in tarball before upload

</specifics>

<deferred>
## Deferred Ideas

None — phase scope is well-defined and bounded by prior phase decisions.

---

*Phase: 12-release-ci-hardening-and-update-url-configuration*
*Context gathered: 2026-04-17*
*Auto-discuss: All gray areas selected with recommended defaults*
