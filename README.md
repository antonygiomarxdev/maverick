# Maverick

Offline-first LoRaWAN **edge runtime** (v1 baseline): small core, strict boundaries, optional adapters, future cloud sync via contracts only.

## North star

Keep LoRaWAN operations running locally when connectivity is poor or absent; preserve durable local truth; never couple the kernel to optional infrastructure.

## Docs (source of truth)

| Doc | Purpose |
|-----|---------|
| [ROADMAP.md](ROADMAP.md) | Now / Next / Later execution board |
| [docs/00-product-intent.md](docs/00-product-intent.md) | Scope and non-negotiables |
| [docs/01-execution-plan.md](docs/01-execution-plan.md) | Slices and sprint plan |
| [docs/03-operating-model.md](docs/03-operating-model.md) | Focus, KPIs, testing gates |
| [docs/runbook-edge.md](docs/runbook-edge.md) | Field visibility and ops |

## Workspace layout

```
crates/
  maverick-domain/              # entities & value objects (no I/O)
  maverick-core/                # use cases, ports, LoRaWAN 1.0.x Class A capability module
  maverick-runtime-edge/        # binary: maverick-edge
  maverick-adapter-radio-udp/   # RadioTransport adapter (stub)
  maverick-extension-contracts/ # sync envelope contracts (v1.x)
  maverick-cloud-core/          # hub HubSyncIngest port
  maverick-integration-tests/    # integration smoke tests
```

## Quick start

```bash
cargo build --workspace
cargo test --workspace
cargo run -p maverick-runtime-edge --bin maverick-edge -- health
cargo run -p maverick-runtime-edge --bin maverick-edge -- status
```

## License

MIT — see [LICENSE](LICENSE).
