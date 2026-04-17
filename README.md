# Maverick

**LoRaWAN. Offline. Always.**

*Self-contained LoRaWAN gateway + network server for edge deployments where connectivity is unreliable or nonexistent.*

[Install](#install) · [Quick Start](#quick-start) · [Why Maverick](#why-maverick) · [Extensions](#extensions) · [Community](#community)

---

## What is this?

Maverick is a **gateway and LoRaWAN network server in one binary**. It runs on a Raspberry Pi in the middle of a cornfield, reads packets from a SX1302/SX1303 radio over SPI, stores everything in local SQLite — and keeps working even when the satellite link goes down for three days.

No cloud required. No external dependencies. Your data is on that device until you decide otherwise.

```
┌──────────────────────────────────────────────────────────┐
│  Raspberry Pi (edge)                                      │
│  ┌────────────────┐     ┌─────────────────────────────┐  │
│  │   SX1303 HAT    │────▶│        maverick-edge        │  │
│  │   (radio)       │     │  gateway + LNS + SQLite    │  │
│  └────────────────┘     └─────────────────────────────┘  │
│                                      │                   │
│                          Extensions: TUI, HTTP, MQTT, AI│
└──────────────────────────────────────────────────────────┘
                                      │
                                      │ when connected
                                      ▼
                               Maverick Cloud (future)
```

## Why Maverick

| | Traditional LNS | Maverick |
|---|---|---|
| Requires internet | Yes | No |
| Runs on Raspberry Pi | Needs gateway + server | Single binary |
| Data if offline | Lost | Persists locally |
| Extension crash | May affect LNS | Isolated |
| Setup complexity | High | `curl ... | bash` |

Works with existing packet forwarders too (UDP/GWMP), if you already have gateway hardware.

## Quick Start

### Install (one command)

```bash
curl -fsSL https://raw.githubusercontent.com/antonygiomarxdev/maverick/main/scripts/install-linux.sh | bash
```

Or manual:

```bash
# Download from releases
wget https://github.com/antonygiomarxdev/maverick/releases/latest/download/maverick-edge-linux-armv7.tar.gz
tar -xzf maverick-edge-linux-armv7.tar.gz
sudo mv maverick-edge /usr/local/bin/

# Run setup (interactive)
maverick-edge setup
```

### Verify

```bash
maverick-edge health
maverick-edge status
```

### Configure

Edit `/etc/maverick/lns-config.toml` to add your devices and region. Then:

```bash
maverick-edge radio ingest-loop
```

That's it. Uplinks go to SQLite. Connect a dashboard extension when you're ready.

## Extensions

Everything is optional. Default install is just `maverick-edge` — nothing else.

| Extension | When you need it |
|-----------|------------------|
| [`maverick-tui`](https://github.com/antonygiomarxdev/maverick/tree/main/crates/maverick-extension-tui) | Terminal console for device management |
| [`maverick-dashboard`](https://github.com/antonygiomarxdev/maverick) (future) | Web UI for visualization |
| [`maverick-http`](https://github.com/antonygiomarxdev/maverick) (future) | Forward uplinks via HTTP webhooks |
| [`maverick-mqtt`](https://github.com/antonygiomarxdev/maverick) (future) | Publish to MQTT broker |
| [`maverick-ai`](https://github.com/antonygiomarxdev/maverick) (future) | Anomaly detection, AI analytics |

Extensions are **separate processes**. If one crashes, `maverick-edge` keeps running.

## Architecture

```
maverick-edge
┌────────────────────────────────────────────────────────────┐
│  Radio SPI  │  SQLite  │  CLI  │  Extension IPC           │
│  (SX1302/3) │ (local)  │       │  (HTTP, Unix socket)     │
└────────────────────────────────────────────────────────────┘
       │                                      │
       ▼                                      ▼
  LoRa Frames                          Extensions
  (uplinks ↓                           (optional)
   downlinks ↑)
```

## Status

**Public beta** — v0.x. Core ingest path works. Extensions are being built.

Roadmap: [ROADMAP.md](ROADMAP.md)

## Install Options

| Method | Use case |
|--------|----------|
| [`install.sh`](scripts/install-linux.sh) | Production deployments |
| [Docker](docker-compose.yml) | Try it locally |
| [Source build](#build-from-source) | Development |

### Build from source

```bash
git clone https://github.com/antonygiomarxdev/maverick
cd maverick
cargo build --release -p maverick-runtime-edge
./target/release/maverick-edge --version
```

### Docker (local testing)

```bash
docker compose up
```

## Community

- **Issues**: [GitHub Issues](https://github.com/antonygiomarxdev/maverick/issues)
- **Discussions**: [GitHub Discussions](https://github.com/antonygiomarxdev/maverick/discussions)
- **Contributing**: [CONTRIBUTING.md](CONTRIBUTING.md)

Contributions welcome: core, extensions, hardware compatibility, docs.

## Related

| Project | How it relates |
|---------|----------------|
| [ChirpStack](https://www.chirpstack.io/) | LNS, requires PostgreSQL + MQTT + internet |
| [The Things Stack](https://www.thethingsnetwork.org/docs/lns/) | LNS, cloud-first |
| [Helium](https://www.helium.com/) | Decentralized wireless, depends on hotspot network |
| **Maverick** | Offline-first, local, self-contained |

---

## License

MIT — see [LICENSE](LICENSE)
