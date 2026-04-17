# Phase 12: Release CI hardening and update URL configuration — Summary

**Phase:** 12-release-ci-hardening-and-update-url-configuration
**Completed:** 2026-04-17
**Plans:** 5 | **Tasks:** All complete

## What Was Built

### Release Infrastructure Hardening

Phase 12 completed the following release infrastructure improvements:

1. **Updated maverick.toml release_url** - Configured the auto-update mechanism to use GitHub Releases URL
2. **Added SBOM generation** - Syft-based SPDX JSON SBOM for each release artifact
3. **Added SIGSTORE code signing** - Keyless binary signing using SIGSTORE for authenticity verification
4. **Created SECURITY.md** - Vulnerability disclosure policy with 7-day response timeline
5. **Added version.txt to releases** - Auto-update mechanism now has version file in each tarball

## Key Files Modified

| File | Change |
|------|--------|
| `etc/maverick.toml` | Updated `release_url` to GitHub Releases |
| `.github/workflows/release.yml` | Added SBOM, SIGSTORE signing, version.txt |
| `SECURITY.md` | Created/updated vulnerability disclosure policy |

## Verification

- `release_url` points to `https://github.com/antonygiomarxdev/maverick/releases/download`
- SBOM generated via `anchore/sbom-action@v0` with SPDX JSON format
- SIGSTORE signing via `sigstore/sigstore-github-actions/sign-blobs@main`
- version.txt created and included in release tarballs
- SECURITY.md contains coordinated disclosure policy with 7-day response timeline

## Notes

- SIGSTORE uses keyless signing via OIDC - no long-term key management required
- SBOM and signature artifacts are attached to GitHub releases automatically
- All changes are backward compatible with existing update mechanism
