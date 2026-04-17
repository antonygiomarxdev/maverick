---
phase: 12
name: release-ci-hardening-and-update-url-configuration
wave: 1
depends_on: []
requirements_addressed: []
files_modified:
  - etc/maverick.toml
  - .github/workflows/release.yml
  - SECURITY.md
autonomous: true
---

## Plan 01: Update maverick.toml release URL

<read_first>
- etc/maverick.toml
</read_first>

<objective>
Update the release_url in maverick.toml to point to GitHub Releases
</objective>

<action>
Change the `release_url` setting in `etc/maverick.toml` under `[update]` section from:
```
release_url = ""
```
to:
```
release_url = "https://github.com/antonygiomarxdev/maverick/releases/download"
```
</action>

<acceptance_criteria>
- grep "release_url" etc/maverick.toml shows `release_url = "https://github.com/antonygiomarxdev/maverick/releases/download"`
</acceptance_criteria>

---

## Plan 02: Add SBOM generation to release workflow

<read_first>
- .github/workflows/release.yml
</read_first>

<objective>
Add Syft-based SBOM generation to the release workflow for SPDX-compliant software bill of materials
</objective>

<action>
Add a new step in the `build` job after the "Build release binary" step:

```yaml
      - name: Generate SBOM
        uses: anchore/sbom-action@v0
        with:
          image: ${{ matrix.target }}
          format: spdx-json
          output-file: sbom-${{ matrix.target }}.spdx.json

      - name: Upload SBOM artifact
        uses: actions/upload-artifact@v4
        with:
          name: sbom-${{ matrix.artifact }}
          path: sbom-${{ matrix.target }}.spdx.json
```

In the `publish-github-release` job, add download and attach SBOM artifacts:
```yaml
      - name: Download SBOM artifacts
        uses: actions/download-artifact@v4
        with:
          path: sbom-artifacts/
          pattern: sbom-*
          merge-multiple: true

      - name: Attach SBOM to release
        uses: softprops/action-gh-release@v2
        with:
          # ... existing files: ...
          files: |
            artifacts/**/*.tar.gz
            artifacts/**/*.sha256
            sbom-artifacts/*.spdx.json
```
</action>

<acceptance_criteria>
- grep "sbom-action" .github/workflows/release.yml returns the SBOM generation step
- grep "spdx-json" .github/workflows/release.yml returns the format specification
- grep "sbom-artifacts" .github/workflows/release.yml shows SBOM files attached to release
</acceptance_criteria>

---

## Plan 03: Add SIGSTORE code signing to release workflow

<read_first>
- .github/workflows/release.yml
</read_first>

<objective>
Add keyless code signing using SIGSTORE for binary authenticity verification
</objective>

<action>
Add SIGSTORE signing step in the `build` job after the "Package binary" step:

```yaml
      - name: Sign binaries with SIGSTORE
        uses: sigstore/sigstore-github-actions/sign-blobs@main
        with:
          blobs: |
            ${{ matrix.artifact }}.tar.gz
            ${{ matrix.artifact }}.tar.gz.sha256
          output-signatures: "${{ matrix.artifact }}.tar.gz.sig"
        env:
          SIGSTORE_REKOR_URL: https://rekor.sigstore.dev
          SIGSTORE_OIDC_ISSUER: https://oauth2.sigstore.dev
          SIGSTORE_OIDC_CLIENT_ID: sigstore

      - name: Upload signatures
        uses: actions/upload-artifact@v4
        with:
          name: signatures-${{ matrix.artifact }}
          path: "*.sig"
```

In the `publish-github-release` job, download and attach signature files:
```yaml
      - name: Download signature artifacts
        uses: actions/download-artifact@v4
        with:
          path: signature-artifacts/
          pattern: signatures-*
          merge-multiple: true

      - name: Attach signatures to release
        uses: softprops/action-gh-release@v2
        with:
          # ... existing files: ...
          files: |
            artifacts/**/*.tar.gz
            artifacts/**/*.sha256
            sbom-artifacts/*.spdx.json
            signature-artifacts/*.sig
```
</action>

<acceptance_criteria>
- grep "sigstore/sigstore-github-actions" .github/workflows/release.yml returns the signing action
- grep "output-signatures" .github/workflows/release.yml returns signature output configuration
- grep "signature-artifacts" .github/workflows/release.yml shows signatures attached to release
</acceptance_criteria>

---

## Plan 04: Create SECURITY.md vulnerability disclosure policy

<read_first>
- None (new file)
</read_first>

<objective>
Create SECURITY.md with coordinated vulnerability disclosure policy
</objective>

<action>
Create `SECURITY.md` at repository root with the following content:

```markdown
# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 1.x     | :white_check_mark: |
| < 1.0   | :x:                |

## Reporting a Vulnerability

We take security issues seriously. If you discover a security vulnerability, please report it responsibly.

### How to Report

**Please DO NOT file a public GitHub issue for security vulnerabilities.**

Instead, please report them through GitHub's private vulnerability reporting system:

1. Go to the repository's **Security** tab
2. Click **"Report a vulnerability"**
3. Fill out the vulnerability report form

Alternatively, you can send a detailed email to the maintainers.

### What to Include

Please include as much of the following as possible:

- Description of the vulnerability
- Steps to reproduce the issue
- Potential impact of the vulnerability
- Any suggested fixes (optional)

### Response Timeline

- **Initial Response**: We aim to acknowledge reports within 7 days
- **Status Update**: We will provide a timeline for when we expect to have a fix
- **Disclosure**: After the vulnerability is fixed, we will publish a security advisory

### Scope

This security policy applies to:
- LoRaWAN protocol handling and frame validation
- MIC (Message Integrity Code) verification
- Session key (NwkSKey, AppSKey) storage and handling
- Device authentication and session management
- Downlink scheduling and queue management

### Out of Scope

- Social engineering attacks
- Physical security of hardware devices
- Network-level attacks (DDoS, MITM on upstream connections)
- Issues in third-party dependencies (report to upstream maintainers)

## Security Updates

Security updates will be released as patch versions (e.g., 1.0.1) and announced through:
- GitHub Security Advisories
- Release notes

---

*Last updated: 2026-04-17*
```
</action>

<acceptance_criteria>
- test -f SECURITY.md returns 0 (file exists)
- grep "Report a vulnerability" SECURITY.md returns the reporting instructions
- grep "LoRaWAN protocol handling" SECURITY.md returns the scope section
- grep "7 days" SECURITY.md returns the response timeline
</acceptance_criteria>

---

## Plan 05: Add version.txt to release artifacts

<read_first>
- .github/workflows/release.yml
</read_first>

<objective>
Create version.txt file in each release artifact tarball for auto-update mechanism
</objective>

<action>
In the `build` job, after the "Package binary" step and before SHA256 calculation, add:

```yaml
      - name: Create version.txt
        run: |
          echo "${{ needs.prepare.outputs.version_tag }}" > version.txt
          echo "${{ needs.prepare.outputs.version_tag }}" > maverick-version.txt

      - name: Update tarball with version.txt
        run: |
          tar -czf ${{ matrix.artifact }}.tar.gz.new maverick-edge maverick-edge-tui install-linux.sh version.txt
          mv ${{ matrix.artifact }}.tar.gz.new ${{ matrix.artifact }}.tar.gz
```

Also update the verification step to expect version.txt:
```yaml
      - name: Verify packaged archive contents
        shell: bash
        run: |
          set -euo pipefail
          archive="${{ matrix.artifact }}.tar.gz"
          tar -tzf "${archive}" | sort > archive.txt
          diff -u <(printf '%s\n' install-linux.sh maverick-edge maverick-edge-tui version.txt | sort) archive.txt
```
</action>

<acceptance_criteria>
- grep "Create version.txt" .github/workflows/release.yml returns the version creation step
- grep "maverick-version.txt" .github/workflows/release.yml returns the version file name
- grep "version.txt" .github/workflows/release.yml shows it added to tarball
- grep "Verify packaged archive contents" .github/workflows/release.yml shows version.txt in expected files
</acceptance_criteria>

---

## Verification

<must_have>
- etc/maverick.toml has correct release_url pointing to GitHub Releases
- release.yml has Syft SBOM generation step
- release.yml has SIGSTORE code signing step
- release.yml attaches both SBOM and signatures to GitHub Release
- SECURITY.md exists with vulnerability reporting policy
- version.txt is created and included in release tarballs
</must_have>

<verification_commands>
- grep "release_url = \"https://github.com/antonygiomarxdev/maverick/releases/download\"" etc/maverick.toml
- grep "sbom-action" .github/workflows/release.yml
- grep "sigstore/sigstore-github-actions" .github/workflows/release.yml
- test -f SECURITY.md
- grep "version.txt" .github/workflows/release.yml
</verification_commands>
