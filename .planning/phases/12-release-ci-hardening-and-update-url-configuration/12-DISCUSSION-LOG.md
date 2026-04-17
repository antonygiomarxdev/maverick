# Phase 12: Release CI hardening and update URL configuration - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-04-17
**Phase:** 12-release-ci-hardening-and-update-url-configuration
**Areas discussed:** SBOM generation, Code signing approach, SECURITY.md policy, Version artifact

---

## SBOM Generation

| Option | Description | Selected |
|--------|-------------|----------|
| Syft | Industry standard SBOM generator, GitHub-native integration | ✓ |
| CycloneDX | XML/JSON SBOM format, more verbose |  |
| Manual | Hand-crafted SBOM, error-prone |  |

**User's choice:** Syft (recommended default)
**Notes:** Auto-selected in --auto mode based on industry standard and GitHub Actions support

---

## Code Signing Approach

| Option | Description | Selected |
|--------|-------------|----------|
| SIGSTORE | Keyless signing via OIDC, no key management overhead | ✓ |
| GPG | Traditional PGP signing, requires key rotation management |  |
| None | Skip code signing |  |

**User's choice:** SIGSTORE (recommended default)
**Notes:** Auto-selected in --auto mode - SIGSTORE is the modern standard for OSS projects

---

## SECURITY.md Policy

| Option | Description | Selected |
|--------|-------------|----------|
| GitHub Security Advisories | Use GHSA for private vulnerability reporting | ✓ |
| Email disclosure | Contact email for reports |  |
| Open bug bounty | Public bug bounty program |  |

**User's choice:** GitHub Security Advisories (recommended default)
**Notes:** Auto-selected in --auto mode - native GitHub integration, private reporting

---

## Version Artifact

| Option | Description | Selected |
|--------|-------------|----------|
| version.txt in tarball | Include version.txt in each release tarball | ✓ |
| Separate artifact | Upload version.txt as separate release asset |  |

**User's choice:** version.txt in tarball (recommended default)
**Notes:** Auto-selected in --auto mode - aligns with Phase 11 auto-update mechanism

---

## Deferred Ideas

None — discussion stayed within phase scope

