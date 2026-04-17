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
