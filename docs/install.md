# Maverick v1 Install (Linux-first)

This is the canonical installation path for v1 local gateway operation.

## Scope

- Supported OS for v1: **Linux only**
- Official install method: **native binary**
- Docker: optional convenience path (not required for small gateways)
- CLI is default; `maverick-edge-tui` is optional extension UX.
- Release tag format: `vX.Y.Z`
- Version rule (v1.x): keep `maverick-edge` and `maverick-edge-tui` on the same tag.

## Distro support policy (public baseline)

Maverick uses support tiers so operators know what is release-gated versus best-effort.

- Tier 1 (release-gated for edge):
  - Raspberry Pi OS Lite (Bookworm, 64-bit) for `aarch64-unknown-linux-gnu`
  - Debian 12 minimal for `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, and `armv7-unknown-linux-gnueabihf`
- Tier 2 (best-effort for edge):
  - Ubuntu Server LTS (22.04/24.04) with matching architecture artifacts
  - Other glibc-based distros where required base tools exist
- Not currently supported for native binaries:
  - musl-only distributions (for example Alpine base images) unless explicitly documented in a future release

Cloud baseline (future, post-v1 runtime focus):

- Candidate Tier 1 cloud distros: Debian 12 LTS and Ubuntu 24.04 LTS on `x86_64`/`arm64`.
- Cloud support becomes Tier 1 only after cloud runtime paths have dedicated CI/test gates.

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

Prerequisite: **at least one [GitHub Release](https://github.com/antonygiomarxdev/maverick/releases)** must exist. The script downloads release assets (`maverick-<target>.tar.gz`); a green `main` branch alone is not enough until a maintainer publishes a release (usually by pushing a version tag).

**Recommended: one command** (downloads and runs the installer; no separate `chmod`):

```bash
curl -fsSL "https://raw.githubusercontent.com/antonygiomarxdev/maverick/main/scripts/install-linux.sh" | bash -s -- --version latest --install-dir /usr/local/bin
```

The installer uses standard Linux DX conventions:

- detects common package managers (`apt-get`, `dnf`, `yum`, `apk`, `pacman`, `zypper`),
- installs missing prerequisites when possible (`tar`, `coreutils`, `ca-certificates`, and similar base tools),
- validates runtime compatibility (glibc baseline) before downloading/installing binaries,
- validates installed binaries with a `--help` smoke check before exiting.

`bash -s --` passes arguments to the script read from stdin. Use `sudo` only if you need elevation for the whole pipeline (for example `sudo` in front of `bash` when installing to `/usr/local/bin` as a non-root user).

**Alternative: save then run** (if you prefer not to pipe to `bash`):

```bash
curl -fsSL "https://raw.githubusercontent.com/antonygiomarxdev/maverick/main/scripts/install-linux.sh" -o /tmp/install-maverick.sh
chmod +x /tmp/install-maverick.sh
/tmp/install-maverick.sh --version latest --install-dir /usr/local/bin
```

Disable automatic prerequisite installation only if you want a stricter/manual environment:

```bash
/tmp/install-maverick.sh --version v0.1.0 --install-dir /usr/local/bin --no-install-deps
```

Bypass runtime compatibility precheck only for advanced troubleshooting:

```bash
/tmp/install-maverick.sh --version v0.1.0 --install-dir /usr/local/bin --skip-runtime-check
```

If `--version latest` fails with a `404` from `curl`, there is no `latest` release yet. Use **Manual install** with an explicit `VERSION="vX.Y.Z"` after the first release is published, or build from source (repository `README.md`).

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

- `ghcr.io/antonygiomarxdev/maverick:latest` points to the latest tagged stable `1.x+` release.
- For deterministic deployments, prefer explicit version tags.

## Troubleshooting

- `curl: (22) ... 404` during `--version latest`: no published GitHub Release yet, or GitHub API rate limit (unauthenticated). Open the [releases page](https://github.com/antonygiomarxdev/maverick/releases); if empty, wait for the first release or build from source.
- `curl: (22) ... 404` when downloading the `.tar.gz`: wrong or unpublished `VERSION`, or asset name mismatch for your architecture.
- `missing required command: ...`: rerun without `--no-install-deps` so the installer can bootstrap the missing prerequisite, or install it manually with your distro package manager.
- `host glibc ... is older than required ...`: your distro baseline is too old for published binaries. The installer cannot safely upgrade glibc in-place; upgrade OS baseline (Bookworm/Debian 12) or build from source.
- `installed maverick-edge failed the --help smoke check`: check the runtime output printed by the installer; common cause is glibc/loader mismatch on older distros. Tier 1 edge baseline is Raspberry Pi OS Lite Bookworm or Debian 12 minimal.
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
