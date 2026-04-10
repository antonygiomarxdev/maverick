# Maverick

Offline-first LoRaWAN **edge runtime** (v1 baseline): small core, strict boundaries, optional adapters, future cloud sync via contracts only.

## North star

Keep LoRaWAN operations running locally when connectivity is poor or absent; preserve durable local truth; never couple the kernel to optional infrastructure.

## Docs (source of truth)

| Doc | Purpose |
|-----|---------|
| [docs/README.md](docs/README.md) | Canonical docs index and repo map |
| [ROADMAP.md](ROADMAP.md) | Now / Next / Later execution board |
| [docs/00-product-intent.md](docs/00-product-intent.md) | Scope and non-negotiables |
| [docs/01-execution-plan.md](docs/01-execution-plan.md) | Slices and sprint plan |
| [docs/03-operating-model.md](docs/03-operating-model.md) | Focus, KPIs, testing gates |
| [docs/runbook-edge.md](docs/runbook-edge.md) | Field visibility and ops |
| [docs/install.md](docs/install.md) | Linux-first install and post-install checks |

## Workspace layout

```
crates/
  maverick-domain/              # entities & value objects (no I/O)
  maverick-core/                # use cases, ports, LoRaWAN 1.0.x Class A capability module
  maverick-extension-tui/       # optional terminal UX extension
  maverick-runtime-edge/        # binary: maverick-edge
  maverick-adapter-persistence-sqlite/ # Session/Uplink/Audit + storage pressure (SQLite)
  maverick-adapter-radio-udp/   # RadioTransport adapter (stub)
  maverick-extension-contracts/ # sync envelope contracts (v1.x)
  maverick-cloud-core/          # hub HubSyncIngest port
  maverick-integration-tests/    # integration smoke tests
```

## Quick start (source build)

```bash
cargo build --workspace
cargo test --workspace
cargo run -p maverick-runtime-edge --bin maverick-edge -- health
cargo run -p maverick-runtime-edge --bin maverick-edge -- status
# Optional: set data directory for SQLite (default ./data, file maverick.db)
export MAVERICK_DATA_DIR="./data"
cargo run -p maverick-runtime-edge --bin maverick-edge -- storage-pressure
# UDP downlink probe (resilient RadioTransport); send may succeed even if nothing listens (UDP)
cargo run -p maverick-runtime-edge --bin maverick-edge -- radio downlink-probe --host 127.0.0.1 --port 17000
# One-shot GWMP ingest path through core use case (waits for one UDP packet)
cargo run -p maverick-runtime-edge --bin maverick-edge -- radio ingest-once --bind 0.0.0.0:17000 --timeout-ms 5000
# Supervised gateway loop (bounded by max-messages, continues on recoverable failures)
cargo run -p maverick-runtime-edge --bin maverick-edge -- radio ingest-loop --bind 0.0.0.0:17000 --read-timeout-ms 1000 --max-messages 1000
```

## Install (Linux-first v1)

- Official path: native Linux binary install
- See full guide: [`docs/install.md`](docs/install.md)
- Helper script: `scripts/install-linux.sh`
- Optional extension: `maverick-edge-tui` (CLI remains default)

## Releases, tags, and packages

- Release tags use the format `vX.Y.Z` (example: `v0.1.0`).
- Each tag publishes Linux tarballs with `maverick-edge`, `maverick-edge-tui`, and checksum files.
- Container images are published to `ghcr.io/antonygiomarxdev/maverick`.
- During v1.x, core and extension binaries are version-locked by release tag.

See policy details in [`docs/extensions.md`](docs/extensions.md).

## License

MIT — see [LICENSE](LICENSE).
