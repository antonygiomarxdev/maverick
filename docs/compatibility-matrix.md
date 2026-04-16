# Hardware & radio path compatibility matrix

Status: **Active** (2026-04-13)

This document separates what the project **tests in hardware** from what is **expected to work** from architecture alone. It complements distro tiers in [`install.md`](install.md).

## Support levels

| Level | Meaning |
|--------|---------|
| **Tested** | Maintainer or community provided **reproducible evidence** (see template below); listed in this matrix. |
| **Theoretical** | Fits the architecture (Linux + Semtech GWMP `PUSH_DATA` over UDP to the configured bind) but **no** formal hardware gate yet. |
| **Not supported** | Out of scope for v1 (documented explicitly when we know it breaks). |

## Platform & uplink path

| Platform / gateway path | Uplink ingest | Level | Notes |
|-------------------------|---------------|-------|--------|
| Linux `aarch64` / `x86_64` / `armv7` | GWMP UDP (`maverick-edge radio ingest-*`, bind via `MAVERICK_GWMP_BIND`) | Theoretical → **Tested** when evidenced | Default v1 path; packet forwarder must target the same UDP bind. |
| RAK Pi + RAK concentrator HAT + standard packet forwarder | GWMP UDP | Tested (when gated) | Use [`scripts/e2e-rakpi-prepush.sh`](../scripts/e2e-rakpi-prepush.sh) before pushing changes that touch ingest/runtime. |
| Non-Linux | — | Not supported | Native binaries are Linux-only for v1. |

## Evidence template (community / maintainers)

Open a PR or discussion with:

1. **Hardware**: board model, radio/concentrator, OS image (`uname -a`, distro release).
2. **Binaries**: `maverick-edge --version` (or build id) and same tag for optional `maverick-edge-tui`.
3. **Config**: redacted `/etc/maverick/lns-config.toml` shape (schema version, app/device count only if sensitive).
4. **Runtime**: `maverick-edge probe`, `status`, `health` (JSON or summarized).
5. **RF proof**: at least one uplink observed with `ingest-once` or `ingest-loop` counters showing `ingested > 0`, plus SQLite row expectation if applicable.
6. **Repeatability**: exact `MAVERICK_*` env vars and systemd unit snippet if used.

Maintainers may promote a row from **Theoretical** to **Tested** when evidence is verified.

## Related commands

- `maverick-edge probe` — `RuntimeCapabilityReport` (hardware, radio hints, selected GWMP bind, snapshot id).
- `maverick-edge status` — includes `runtime_capabilities` object.
- `maverick-edge health` — includes `radio_environment` component.
