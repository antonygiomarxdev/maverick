# Maverick v1 Install (Linux-first)

This is the canonical installation path for v1 local gateway operation.

## Scope

- Supported OS for v1: **Linux only**
- Official install method: **native binary**
- Docker: optional convenience path (not required for small gateways)
- CLI is default; `maverick-edge-tui` is optional extension UX.
- Release tag format: `vX.Y.Z`
- Version rule (v1.x): keep `maverick-edge` and `maverick-edge-tui` on the same tag.

## Choose your architecture

Release assets are published per Linux target:

- `x86_64-unknown-linux-gnu` -> `maverick-x86_64-unknown-linux-gnu.tar.gz`
- `aarch64-unknown-linux-gnu` -> `maverick-aarch64-unknown-linux-gnu.tar.gz`
- `armv7-unknown-linux-gnueabihf` -> `maverick-armv7-unknown-linux-gnueabihf.tar.gz`

Auto-detect on host:

```bash
uname -m
```

## Quick install (recommended)

Use the helper script from this repo:

```bash
curl -fsSL "https://raw.githubusercontent.com/antonygiomarxdev/maverick/main/scripts/install-linux.sh" -o /tmp/install-maverick.sh
chmod +x /tmp/install-maverick.sh
/tmp/install-maverick.sh --version latest --install-dir /usr/local/bin
```

## Manual install

Set your version and architecture:

```bash
VERSION="v0.1.0"
ARCH="x86_64-unknown-linux-gnu" # or aarch64-unknown-linux-gnu / armv7-unknown-linux-gnueabihf
ASSET="maverick-${ARCH}.tar.gz"
SHA="${ASSET}.sha256"
BASE="https://github.com/antonygiomarxdev/maverick/releases/download/${VERSION}"
```

Download and verify checksum:

```bash
curl -fsSL "${BASE}/${ASSET}" -o "${ASSET}"
curl -fsSL "${BASE}/${SHA}" -o "${SHA}"
sha256sum -c "${SHA}"
```

Install:

```bash
tar -xzf "${ASSET}"
chmod +x maverick-edge
sudo mv maverick-edge /usr/local/bin/maverick-edge
chmod +x maverick-edge-tui
sudo mv maverick-edge-tui /usr/local/bin/maverick-edge-tui
```

## Post-install smoke checks

```bash
maverick-edge --help
maverick-edge status
maverick-edge health
MAVERICK_DATA_DIR="./data" maverick-edge storage-pressure
maverick-edge-tui config-show
```

Gateway ingest loop sanity check:

```bash
MAVERICK_GWMP_BIND="0.0.0.0:17000" \
MAVERICK_GWMP_LOOP_READ_TIMEOUT_MS="1000" \
MAVERICK_GWMP_LOOP_MAX_MESSAGES="1000" \
maverick-edge radio ingest-loop --bind 0.0.0.0:17000 --read-timeout-ms 1000 --max-messages 1000
```

## Optional Docker path

If your gateway has enough resources and a container runtime:

```bash
docker run --rm -it \
  -e MAVERICK_DATA_DIR=/var/lib/maverick \
  -e MAVERICK_GWMP_BIND=0.0.0.0:17000 \
  -v maverick_data:/var/lib/maverick \
  -p 17000:17000/udp \
  ghcr.io/antonygiomarxdev/maverick:latest \
  maverick-edge radio ingest-loop --bind 0.0.0.0:17000 --read-timeout-ms 1000 --max-messages 1000
```

Docker tag notes:

- `ghcr.io/antonygiomarxdev/maverick:latest` points to the latest tagged stable release.
- For deterministic deployments, prefer explicit version tags.

## Troubleshooting

- `command not found`: ensure `/usr/local/bin` is in `PATH`.
- `sha256sum mismatch`: re-download both asset and checksum; do not install.
- `permission denied` on bind: use non-privileged UDP port or run with proper capabilities.
- SQLite open failure: check write permissions in `MAVERICK_DATA_DIR`.

## Optional TUI extension

`maverick-edge-tui` is an optional terminal extension that wraps common operator flows:

- Welcome and essentials configuration
- Status/health helpers
- Ingest-loop launch using saved config

Run:

```bash
maverick-edge-tui
```
