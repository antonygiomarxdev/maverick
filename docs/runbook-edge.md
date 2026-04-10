# Edge Runtime Runbook (v1 baseline)

## Local visibility (no external stack)

1. `maverick-edge status` — suggested install profile and memory hint.
2. `maverick-edge health` — JSON health snapshot from local probes.
3. `maverick-edge recent-errors` — placeholder until log tail is wired to rotating files.
4. `maverick-edge probe` — hardware capability JSON for support.
5. `maverick-edge storage-policy <profile>` — effective `StoragePolicy` JSON.

## Degradation signals

- `HealthStatus::Degraded` when probes return incomplete data (e.g. zero memory).
- Storage pressure levels map to operator action in persistence layer (future slice).

## Failure handling principle

Recoverable faults must not require manual process restart. If restart is needed, treat as defect unless documented as non-recoverable.
