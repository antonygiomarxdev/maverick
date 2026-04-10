# Maverick Docs Index

Date: 2026-04-10
Status: Active

## Canonical set (authoritative)

These files define the current official direction:

1. `docs/00-product-intent.md`
2. `docs/01-execution-plan.md`
3. `docs/02-delivery-checklist.md`
4. `docs/03-operating-model.md`
5. `docs/04-sprint-1-executable-plan.md`
6. `docs/runbook-edge.md`
7. `docs/05-test-program.md`
8. `docs/code-review-checklist.md` — PR/sprint gate (clean code, SOLID/hexagonal, no magic values).

## Slice evidence (non-canonical supplements)

- `docs/slice-2-evidence.md` — persistence/retention slice outcomes and verification notes.
- `docs/slice-3-evidence.md` — transport resilience (UDP downlink probe, circuit breaker, tests).
- `docs/slice-4-evidence.md` — GWMP ingest path, circuit transition observability, and transport realism tests.

If any other document conflicts with these, the canonical set wins.

## Direction summary

Maverick v1 is a runtime-first, offline-first edge LNS with:

1. strict architectural decoupling,
2. resilience-first runtime behavior,
3. no mandatory cloud dependency in runtime,
4. sync contracts prepared for later versions.

## Repository map

- `crates/maverick-domain` — domain types only
- `crates/maverick-core` — use cases, ports, protocol capability modules, storage policy
- `crates/maverick-runtime-edge` — edge binary `maverick-edge` (CLI visibility baseline)
- `crates/maverick-adapter-persistence-sqlite` — durable SQLite adapter for core ports + storage pressure
- `crates/maverick-adapter-radio-udp` — UDP adapter with resilient transport + GWMP uplink parsing
- `crates/maverick-extension-contracts` — versioned sync/event envelopes for v1.x
- `crates/maverick-cloud-core` — hub-side sync ingest port (no edge dependency)
- `crates/maverick-integration-tests` — cross-crate smoke tests
