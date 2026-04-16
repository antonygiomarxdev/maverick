# Maverick v1 Install (Linux-first)

This is the canonical installation path for v1 local gateway operation.

## Scope

- Supported OS for v1: **Linux only**
- Official install method: **native binary**
- Docker: optional convenience path (not required for small gateways)
- CLI is default; the **Maverick console** (`maverick`, binary `maverick-edge-tui`) is optional extension UX.
- Release tag format: `vX.Y.Z`
- Version rule (v1.x): keep `maverick-edge` and `maverick-edge-tui` on the same tag.

## Hardware & RF path policy

See **[`compatibility-matrix.md`](compatibility-matrix.md)** for concentrator / gateway **tested vs theoretical** listings and the evidence template for community reports. Distro tiers below cover **OS images**; the matrix covers **radio plumbing** (e.g. GWMP UDP to `MAVERICK_GWMP_BIND`).

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

### First-run onboarding (built into the installer)

After binaries are installed, the script runs a **guided wizard** on interactive TTYs (unless `--skip-onboarding`):

1. Confirm **data directory** (`MAVERICK_DATA_DIR`, default `/var/lib/maverick`).
2. Confirm **GWMP bind** and loop policy (`MAVERICK_GWMP_BIND`, read timeout, max messages; `MAVERICK_GWMP_LOOP_MAX_MESSAGES=0` means unlimited receive iterations — default for **systemd** ingest-loop).
3. **Smoke checks** (`maverick-edge --help`, `status`, `health`) with your chosen env.
4. **Extensions**: Maverick console (available); HTTP/MQTT shown as *coming soon* only.
5. **LNS declarative config**: seed `/etc/maverick/lns-config.toml` (`maverick-edge config init`) and sync to SQLite (`config load`). See [`lns-config.md`](lns-config.md).
6. **systemd**: installer writes `/etc/systemd/system/maverick-edge.service`, enables and starts **`maverick-edge radio ingest-loop`** when `systemctl` is available.

Persisted files:

| File | Purpose |
|------|---------|
| `/etc/maverick/runtime.env` | Canonical env for operators and services (`source`able) |
| `/etc/maverick/setup.json` | Onboarding state, selected extensions, installer metadata |
| `/etc/maverick/lns-config.toml` | Operator-edited LNS config (`schema_version = 1`: applications, OTAA/ABP devices); synced to SQLite with `maverick-edge config load` |
| `~/.config/maverick/tui-config.json` | Console extension mirror of runtime (user-writable) |
| `~/.config/maverick/console.toml` | Non-critical console preferences (theme, etc.) |

**Non-interactive / automation** (`--non-interactive`):

Uses environment when set, otherwise these defaults:

- `MAVERICK_DATA_DIR=/var/lib/maverick`
- `MAVERICK_GWMP_BIND=0.0.0.0:17000`
- `MAVERICK_GWMP_LOOP_READ_TIMEOUT_MS=1000`
- `MAVERICK_GWMP_LOOP_MAX_MESSAGES=1000`
- `MAVERICK_INSTALL_CONSOLE=0` — set to `1` to mark the console as enabled in `setup.json` and sync user config.

**Flags:** `--interactive` (default on TTY), `--non-interactive`, `--yes` (accept defaults in prompts), `--skip-onboarding` (binaries only).

The installer creates a **`maverick` → `maverick-edge-tui` symlink** in the install directory when the console binary exists (public command name `maverick`; `maverick-edge-tui` remains the technical binary name during beta).

**Naming (OSS-safe):** public docs and UX use **Maverick** / **`maverick`**; the legacy binary name `maverick-edge-tui` is still shipped and shows a deprecation notice when invoked directly.

### Preview on hardware before publishing (no GitHub release)

To test **exact CI-style** `aarch64` binaries on a Raspberry Pi **without** pushing or creating a release:

1. On a machine with **Docker** (Docker Desktop on Windows is fine), from the repo root:
   ```bash
   docker run --rm -v "$(pwd):/workspace" -w /workspace rust:1-bookworm \
     bash /workspace/scripts/build-linux-aarch64-preview.sh
   ```
   Artifacts: `dist/pi-preview/maverick-edge` and `maverick-edge-tui`.

2. Copy to the gateway and install **without** downloading from GitHub:
   ```bash
   scp -r dist/pi-preview scripts/install-linux.sh pi@rak:/tmp/maverick-preview-src/
   ssh pi@rak "chmod +x /tmp/maverick-preview-src/install-linux.sh && \
     MAVERICK_INSTALL_CONSOLE=1 bash /tmp/maverick-preview-src/install-linux.sh \
     --local-dist-dir /tmp/maverick-preview-src/pi-preview --non-interactive --install-dir /usr/local/bin"
   ```

`--local-dist-dir` skips the release download and uses the supplied directory (must contain `maverick-edge`; `maverick-edge-tui` optional but recommended).

### Hardware validation (e.g. RAK Pi)

Use this checklist on a real gateway (SSH as an operator):

1. `curl -fsSL .../install-linux.sh | bash -s -- --version latest --install-dir /usr/local/bin` (or a pinned `--version vX.Y.Z`).
2. Complete the 4-step wizard; confirm `/etc/maverick/runtime.env` and `setup.json` exist.
3. `maverick-edge status` and `maverick-edge health` with `MAVERICK_DATA_DIR` set as in `runtime.env`.
4. If the console is enabled: `maverick` (or `maverick-edge-tui`) opens the menu; `?` / on-screen hints match `docs/maverick-console-ux-spec.md`.
5. Optional: `--non-interactive` smoke on a second host: `MAVERICK_INSTALL_CONSOLE=1 bash .../install-linux.sh --non-interactive`.

Document gaps (HTTP/MQTT real transports) as *coming soon* — they must not appear operational in the UI.

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
- `host glibc ... is older than required ...`: your distro baseline is too old for published binaries. The installer will not upgrade the OS or runtime libraries automatically. Choose explicitly: build from source, continue with `--skip-runtime-check` (advanced), or plan an OS upgrade to a supported baseline.
- Example: Raspbian 10 (buster) with `glibc 2.28` will fail against current release binaries (requires newer glibc symbols such as `GLIBC_2.34`). In this scenario, keep operator control: either build from source on that host or schedule a planned OS upgrade window.
- `installed maverick-edge failed the --help smoke check`: check the runtime output printed by the installer; common cause is glibc/loader mismatch on older distros. Tier 1 edge baseline is Raspberry Pi OS Lite Bookworm or Debian 12 minimal.
- `command not found`: ensure `/usr/local/bin` is in `PATH`.
- `sha256sum mismatch`: re-download both asset and checksum; do not install.
- `permission denied` on bind: use non-privileged UDP port or run with proper capabilities.
- **`unable to open database file` / SQLite open failure under `/var/lib/maverick`:** the directory was probably created by **root** (installer with `sudo`) while you run `maverick-edge` as a normal user (e.g. `pi`). **Fix:** `sudo chown -R "$(whoami)" /var/lib/maverick` (or set `MAVERICK_DATA_DIR` to a directory you own, e.g. `/home/pi/maverick-data`). Current installers run `chown` to the user who invoked `sudo` (`SUDO_USER`) when creating the data directory; re-run onboarding or apply `chown` once on existing systems.

## Optional Maverick console extension

The console (`maverick`, technical binary `maverick-edge-tui`) is an optional terminal extension that wraps common operator flows:

- Welcome and essentials configuration
- Status/health helpers
- Ingest-loop launch using saved config
- Reads `/etc/maverick/runtime.env` and `setup.json` when present so installer and console stay aligned

Run:

```bash
maverick
# legacy (deprecated name, same binary):
# maverick-edge-tui
```
