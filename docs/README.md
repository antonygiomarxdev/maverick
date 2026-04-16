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
9. `docs/install.md` — Linux-first install path (binary-first DX) plus distro support tiers (edge now, cloud baseline later).
10. `docs/extensions.md` — extension model, compatibility, and version-lock policy.
11. `docs/lns-config.md` — declarative `/etc/maverick/lns-config.toml` schema and `maverick-edge config` commands.
12. `docs/release-policy.md` — tag/version strategy, release checklist, and package publication rules.
13. `scripts/verify-release-cross-builds.sh` — optional Docker smoke build for aarch64/armv7 before tagging (matches Release workflow).
14. `docs/install-console-ux-contract.md` — flow contract/index separating core onboarding vs optional console UX.
15. `docs/install-onboarding-ux-spec.md` — mandatory first-run onboarding UX spec (installer/CLI core path).
16. `docs/maverick-console-ux-spec.md` — optional `maverick` console extension UX spec.
17. `docs/compatibility-matrix.md` — hardware / radio-path support (**tested** vs **theoretical**) and community evidence template.
18. `scripts/e2e-rakpi-prepush.sh` — optional on-device gate (probe/status/health/config) before pushing ingest/runtime changes.

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
- `crates/maverick-extension-tui` — optional Maverick console (`maverick` / `maverick-edge-tui`)
- `crates/maverick-runtime-edge` — edge binary `maverick-edge` (CLI visibility baseline)
- `crates/maverick-adapter-persistence-sqlite` — durable SQLite adapter for core ports + storage pressure
- `crates/maverick-adapter-radio-udp` — UDP adapter with resilient transport + GWMP uplink parsing
- `crates/maverick-extension-contracts` — versioned sync/event envelopes for v1.x
- `crates/maverick-cloud-core` — hub-side sync ingest port (no edge dependency)
- `crates/maverick-integration-tests` — cross-crate smoke tests
