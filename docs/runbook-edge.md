# Edge Runtime Runbook (v1 baseline)

## Local visibility (no external stack)

Global: `--data-dir` or `MAVERICK_DATA_DIR` (default `data`). Local DB file: `<data-dir>/maverick.db`.

1. `maverick-edge status` — suggested install profile, memory hint, and storage summary when `maverick.db` exists.
2. `maverick-edge health` — JSON health snapshot from local probes plus `storage` component when the DB exists (pressure / open errors).
3. `maverick-edge recent-errors` — placeholder until log tail is wired to rotating files.
4. `maverick-edge probe` — hardware capability JSON for support.
5. `maverick-edge storage-policy <profile>` — effective `StoragePolicy` JSON.
6. `maverick-edge storage-pressure` — JSON `StoragePressureSnapshot` when the DB exists.
7. `maverick-edge radio downlink-probe --host <addr> --port <udp>` — sends a single-byte UDP payload through `ResilientRadioTransport` in `maverick-adapter-radio-udp` (timeout / retry / backoff / circuit breaker). JSON result includes `outcome` (`sent` | `failed`) and optional `detail`. Does **not** start the full uplink kernel loop.

## Degradation signals

- `HealthStatus::Degraded` when probes return incomplete data (e.g. zero memory).
- `HealthStatus::Degraded` for component `storage` when `StoragePressureLevel` is above `Normal` (tier fill or on-disk ratio vs optional total-disk hint).
- `HealthStatus::Unhealthy` for `storage` if the DB file exists but cannot be opened.

## Failure handling principle

Recoverable faults must not require manual process restart. If restart is needed, treat as defect unless documented as non-recoverable.
