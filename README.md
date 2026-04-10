# Maverick

Offline-first LoRaWAN edge runtime focused on reliability in unstable networks.

[![CI](https://github.com/antonygiomarxdev/maverick/actions/workflows/ci.yml/badge.svg)](https://github.com/antonygiomarxdev/maverick/actions/workflows/ci.yml)
[![Release](https://github.com/antonygiomarxdev/maverick/actions/workflows/release.yml/badge.svg)](https://github.com/antonygiomarxdev/maverick/actions/workflows/release.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Status](https://img.shields.io/badge/status-public%20beta-orange.svg)](docs/release-policy.md)

## Project status

Maverick is in **public beta**: behavior and operator surfaces may still change. We **version from every tagged release** using SemVer git tags `v0.x.y`. The `0.x` line means pre-1.0; **`1.0.0` will mark the first “stable” line** once we intentionally graduate.

Install and operate from **GitHub Releases** (or GHCR tags that match the same version), not from arbitrary `main` SHAs, unless you are developing the project.

## Why Maverick

Maverick keeps gateway operations running locally when connectivity is weak or absent. The runtime is designed for small Linux gateways, with strict architectural boundaries and resilient behavior under recoverable failures.

Core principles:

- offline-first local truth,
- no mandatory cloud dependency in the runtime path,
- core/kernel isolated from adapters,
- simple operator surface with CLI by default.

## Current scope (v1 baseline)

- Local edge runtime binary: `maverick-edge`
- Optional terminal extension: `maverick-edge-tui`
- GWMP ingest path with supervised loop mode
- Resilience features (timeouts, retries, backoff, circuit behavior)
- SQLite-backed persistence and storage-pressure visibility
- Release artifacts for Linux targets + GHCR image publication

## Install (Linux-first)

Official path is native Linux binaries from GitHub Releases.

- Full guide: [`docs/install.md`](docs/install.md)
- Installer script: `scripts/install-linux.sh`
- Optional extension: `maverick-edge-tui` (CLI remains default)

Quick install (one command):

```bash
curl -fsSL "https://raw.githubusercontent.com/antonygiomarxdev/maverick/main/scripts/install-linux.sh" | bash -s -- --version latest --install-dir /usr/local/bin
```

Requires a published [GitHub Release](https://github.com/antonygiomarxdev/maverick/releases); if `curl` returns `404`, there is no release yet (use source build below or wait for `v0.x.y`). See [`docs/install.md`](docs/install.md) for a save-then-run alternative if you do not want to pipe to `bash`.

## Quick start (source build)

```bash
cargo build --workspace
cargo test --workspace
cargo run -p maverick-runtime-edge --bin maverick-edge -- health
cargo run -p maverick-runtime-edge --bin maverick-edge -- status
export MAVERICK_DATA_DIR="./data"
cargo run -p maverick-runtime-edge --bin maverick-edge -- storage-pressure
cargo run -p maverick-runtime-edge --bin maverick-edge -- radio ingest-loop --bind 0.0.0.0:17000 --read-timeout-ms 1000 --max-messages 1000
```

## Releases and versioning

- Tags follow `vX.Y.Z` (example: `v0.1.0`; during beta the major stays `0`)
- Each tagged release publishes Linux tarballs with:
  - `maverick-edge`
  - `maverick-edge-tui`
  - checksum files (`.sha256`)
- Container image published to `ghcr.io/antonygiomarxdev/maverick`
- During v1.x, core and extension binaries are version-locked by release tag

Policy details: [`docs/extensions.md`](docs/extensions.md)

Release process and checklist: [`docs/release-policy.md`](docs/release-policy.md)

## Repository map

```text
crates/
  maverick-domain/               # entities and value objects (no I/O)
  maverick-core/                 # use cases and ports
  maverick-runtime-edge/         # maverick-edge binary
  maverick-extension-tui/        # optional maverick-edge-tui binary
  maverick-extension-contracts/  # versioned extension/sync contracts
  maverick-adapter-radio-udp/    # UDP transport + GWMP parser
  maverick-adapter-persistence-sqlite/ # sqlite persistence adapters
  maverick-cloud-core/           # cloud-side core contracts
  maverick-integration-tests/    # cross-crate integration tests
```

## Documentation

Start here:

- [`docs/README.md`](docs/README.md) - canonical docs index
- [`ROADMAP.md`](ROADMAP.md) - now/next/later board
- [`docs/runbook-edge.md`](docs/runbook-edge.md) - operator runbook
- [`docs/01-execution-plan.md`](docs/01-execution-plan.md) - implementation slices

## Contributing

Contributions are welcome. Read:

- [`CONTRIBUTING.md`](CONTRIBUTING.md)
- [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md)
- [`SECURITY.md`](SECURITY.md)

## License

MIT - see [`LICENSE`](LICENSE).
