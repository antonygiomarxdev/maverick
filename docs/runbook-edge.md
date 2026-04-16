# Edge Runtime Runbook (v1 baseline)

## Installation path (v1 Linux)

- Canonical install guide: [`install.md`](install.md)
- Official runtime path: native Linux binary (Docker optional)
- Default operator interface: `maverick-edge` CLI
- Optional extension interface: **Maverick console** (`maverick`; binary `maverick-edge-tui`)
- First-run onboarding writes `/etc/maverick/runtime.env` and `/etc/maverick/setup.json` (see [`install.md`](install.md))
- Release tag format: `vX.Y.Z`
- Version rule (v1.x): run core and extension binaries from the same release tag.

## Local visibility (no external stack)

Global: `--data-dir` or `MAVERICK_DATA_DIR` (default `data`). Local DB file: `<data-dir>/maverick.db`.

1. `maverick-edge status` — suggested install profile, memory hint, storage summary when `maverick.db` exists, and **`runtime_capabilities`** (hardware + radio hints + selected GWMP ingest bind + snapshot id). See [`compatibility-matrix.md`](compatibility-matrix.md).
2. `maverick-edge health` — JSON health snapshot from local probes, optional **`radio_environment`** (ingest bind + forwarder hint count), plus `storage` when the DB exists (pressure / open errors).
3. `maverick-edge recent-errors` — placeholder until log tail is wired to rotating files.
4. `maverick-edge probe` — full **`RuntimeCapabilityReport`** JSON: `hardware`, `radio_environment` (systemd / heuristic forwarder units), `selected_ingest`, and versioned **`capability_snapshot`** (not recomputed per uplink).
5. `maverick-edge storage-policy <profile>` — effective `StoragePolicy` JSON.
6. `maverick-edge storage-pressure` — JSON `StoragePressureSnapshot` when the DB exists.
7. `maverick-edge radio downlink-probe --host <addr> --port <udp>` — sends a single-byte UDP payload through `ResilientRadioTransport` in `maverick-adapter-radio-udp` (timeout / retry / backoff / circuit breaker). JSON result includes `outcome` (`sent` | `failed`) and optional `detail`. Does **not** start the full uplink kernel loop.
8. `maverick-edge radio ingest-once --bind <addr:port> --timeout-ms <n>` — binds a UDP socket, waits for one Semtech `PUSH_DATA` datagram, parses `rxpk` entries, and calls core ingest use-case boundaries. Output reports `received`, `parsed`, `ingested`, and `failed`.
9. `maverick-edge radio ingest-loop --bind <addr:port> --read-timeout-ms <n> --max-messages <n>` — supervised local loop for gateway mode. **`--max-messages 0`** runs until process exit (intended under **systemd**). Otherwise caps UDP receive attempts per run. Continues on recoverable read/parse/ingest failures and emits aggregated counters at the end.
10. **`maverick-edge config …`** — declarative LNS file + SQLite sync (`init`, `validate`, `load`, `show`, `list-apps`, `list-devices`, `list-pending`, `approve-device`, `reject-device`). See [`lns-config.md`](lns-config.md).
11. `maverick` (binary `maverick-edge-tui`) — optional terminal UX for welcome/config/status/health/start-ingest-loop; **`[l] LoRaWAN / LNS`** includes CLI shortcuts plus **guided wizards** for applications, devices, and autoprovision (writes `lns-config.toml`, optional `config load`). Aligns with installer output under `/etc/maverick/`.

### Gateway env variables

- `MAVERICK_GWMP_BIND` (default `0.0.0.0:17000`)
- `MAVERICK_GWMP_INGEST_TIMEOUT_MS` (for one-shot mode)
- `MAVERICK_GWMP_LOOP_READ_TIMEOUT_MS` (supervised loop read timeout)
- `MAVERICK_GWMP_LOOP_MAX_MESSAGES` (`0` = unlimited; upper bound per process when `> 0`)

## Degradation signals

- `HealthStatus::Degraded` when probes return incomplete data (e.g. zero memory).
- `HealthStatus::Degraded` for component `radio_environment` on Linux when **no** packet-forwarder services matched heuristics (informational: GWMP may still work if the forwarder is custom-named).
- `HealthStatus::Degraded` for component `storage` when `StoragePressureLevel` is above `Normal` (tier fill or on-disk ratio vs optional total-disk hint).
- `HealthStatus::Unhealthy` for `storage` if the DB file exists but cannot be opened.

## Pre-push E2E gate (RAK Pi / real gateway)

For changes touching ingest, UDP/GWMP, or SQLite persistence, run **`scripts/e2e-rakpi-prepush.sh`** on a target gateway (or via `RAKPI_SSH=pi@host`) and keep the emitted evidence file for the PR. Full RF proof still requires `ingest-loop` with `ingested > 0`.

## Failure handling principle

Recoverable faults must not require manual process restart. If restart is needed, treat as defect unless documented as non-recoverable.

## Troubleshooting (operator quick actions)

- `bind failed` on ingest commands:
  - verify UDP port is free (`ss -lunp | rg 17000`),
  - verify chosen bind address belongs to host.
- `storage open failed`:
  - validate `MAVERICK_DATA_DIR` exists and is writable,
  - check free disk space and ownership.
- high `failed` counter in `ingest-loop` output:
  - inspect GWMP payload source (malformed or wrong protocol),
  - lower `MAVERICK_GWMP_LOOP_READ_TIMEOUT_MS` if long idle periods cause operational confusion,
  - keep process running; recoverable failures should not terminate the loop.
- no uplinks persisted:
  - confirm **`maverick-edge config load`** ran after editing `/etc/maverick/lns-config.toml` (sessions are keyed by `DevAddr`; **OTAA** devices without `dev_addr` in the file do not get a session until you set one or use **approve-device** / autoprovision flows),
  - `maverick-edge config list-devices` — check `activation_mode` and whether `dev_addr_hex` is present,
  - for unknown devices, check `maverick-edge config list-pending` and autoprovision policy (`config show`),
  - verify `status` / `health` output and database file presence in data dir,
  - under **systemd**, use `journalctl -u maverick-edge.service -f` for runtime `tracing` (including `ingest observation failed` warnings).
