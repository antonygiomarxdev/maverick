# Maverick Roadmap (Focus Reset)

Date: 2026-04-10
Status: Active

## North Star

Deliver a runtime-first, offline-first edge LNS that is reliable in remote environments and does not depend on cloud availability.

## Rules of execution

1. Single-track only (`WIP = 1`).
2. Critical KPIs never regress:
   - edge reliability,
   - architecture integrity.
3. Stop-the-line if a critical KPI regresses.
4. Test coverage is a release gate, not a post-delivery task.

## Now / Next / Later

## Now (only active work)

### Slice 0 - Architecture lock

1. Freeze core/adapters/runtime boundaries.
2. Lock protocol capability-module strategy.
3. Lock install-time profile contract.
4. Lock extension contract compatibility policy.
5. Execute Sprint 1 plan in `docs/04-sprint-1-executable-plan.md`.

Exit evidence:

1. Boundary rules documented and testable.
2. Core compiles without concrete infra dependencies.
3. Reliability and boundary acceptance criteria are explicit.
4. Slice 0 includes test strategy and required test layers for all upcoming slices.

## Next (queued, not active)

### Slice 1 - Core minimum complete LNS

1. LoRaWAN 1.0.x Class A core behavior.
2. Region policy baseline (EU868, US915, AU915, AS923, EU433).
3. Typed invariants and policy tests.

### Slice 2 - Durable persistence and storage policy

1. Durable local persistence by default.
2. Hybrid retention policy (tiered + circular continuity under pressure).
3. Install-time profile presets for constrained and larger hardware.

## Later (frozen backlog)

### Slice 3 - Adapter isolation and transport integration

1. First transport adapter behind contract.
2. Optional management adapter.
3. Isolation guarantees (bounded queues, timeout, backoff, circuit break at boundaries).

### Slice 4 - Field visibility baseline

1. Mandatory local CLI (`status`, `health`, `recent-errors`).
2. Mandatory local structured rotating logs.
3. Optional diagnostics snapshot export.

### Slice 5 - Sync-ready contracts (post-v1 runtime)

1. Future sync contracts and envelope compatibility.
2. Keep v1 runtime cloud-independent.

### Slice 6 - Developer-friendly extensibility

1. Versioned extension contracts.
2. Adapter templates and integration guidance.
3. Hybrid SemVer deprecation/compatibility window in practice.

## Method

Use the operating model defined in `docs/03-operating-model.md`.

## Authoritative references

1. `docs/00-product-intent.md`
2. `docs/01-execution-plan.md`
3. `docs/02-delivery-checklist.md`
4. `docs/03-operating-model.md`
