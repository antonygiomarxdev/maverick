# Maverick Delivery Checklist (Runtime-First v1)

Date: 2026-04-10

Use this checklist before declaring v1 complete.

## Slice 2 incremental evidence

See [`slice-2-evidence.md`](slice-2-evidence.md) for what was proven in the persistence/retention slice (SQLite adapter, retention caps, busy retry test, CLI storage visibility). Remaining checklist items below still apply to **full v1** unless explicitly marked done in review.

## Slice 3 incremental evidence

See [`slice-3-evidence.md`](slice-3-evidence.md) for transport resilience (timeout / retry / backoff / circuit breaker on `RadioTransport`), UDP downlink probe CLI, SQLite persistence SRP split, and fault-oriented adapter tests. Sprint review gate: [`code-review-checklist.md`](code-review-checklist.md).

## Slice 4 incremental evidence

See [`slice-4-evidence.md`](slice-4-evidence.md) for GWMP `PUSH_DATA` parsing, one-shot inbound runtime ingest wiring (`radio ingest-once`), and half-open circuit transition observability.

## v1 local Linux closure gate (binary-first DX)

- [x] Canonical Linux install path documented (`docs/install.md`) with architecture mapping and smoke checks.
- [x] Release workflow publishes Linux binary tarballs with `.sha256` checksum files.
- [x] Runtime exposes supervised gateway ingest mode (`radio ingest-loop`) with CLI/env configuration.
- [x] Recoverable-failure continuity is covered for transport parse/timeout/circuit behavior.
- [x] Operator E2E local flow evidence exists (GWMP parse -> core ingest -> SQLite persistence).

## A. Product lock

- [ ] `docs/00-product-intent.md` accepted as source of truth.
- [ ] V1 scope locked: single runtime, LoRaWAN 1.0.x Class A, 5 regions, no runtime sync.
- [ ] Out-of-scope items are explicit.
- [ ] Protocol growth strategy locked: capability modules, not core rewrites.
- [ ] Install-time profile model locked.

## B. Architecture integrity

- [ ] Core depends only on abstractions and domain/application modules.
- [ ] No infrastructure framework dependency leaks into core.
- [ ] Composition layer contains wiring only.
- [ ] Boundary rules are checked in code review and tests.
- [ ] Extension contract policy follows hybrid SemVer window (no breaking in v1.x + documented deprecation window).
- [ ] Clean Architecture and dependency inversion rules are explicitly validated in review.

## C. Reliability baseline

- [ ] Critical ingest loop survives recoverable faults without process restart.
- [ ] Timeout/backoff/circuit-break behavior exists on I/O boundaries.
- [ ] All critical in-memory queues and buffers are bounded.
- [ ] Failure-mode tests exist for DB busy, timeout, malformed input, burst pressure.
- [ ] Runtime behavior matches selected install profile under pressure scenarios.

## D. Persistence and storage pressure

- [ ] Local durable persistence is default.
- [ ] Hybrid retention policy is configurable.
- [ ] Hard-limit behavior keeps runtime alive via circular rollover.
- [ ] Health/alerts expose rollover pressure and history loss risk.

## E. Adapter isolation

- [ ] Transport adapter failures cannot take down core operation.
- [ ] Optional adapters can be disabled with core still operational.
- [ ] No runtime plugin loading is required for v1.

## F. Observability baseline

- [ ] Health state machine (`Healthy`, `Degraded`, `Unhealthy`) is implemented.
- [ ] Minimal pressure and fault metrics are exposed.
- [ ] Operator runbook exists for top edge failure scenarios.
- [ ] Local CLI supports `status`, `health`, and `recent-errors`.
- [ ] Local diagnostics journal is available with bounded retention.
- [ ] Exportable diagnostics snapshot is optional and available on-demand.

## G. Sync readiness (without v1 runtime sync)

- [ ] Sync-related contracts are defined for v1.x.
- [ ] Runtime remains cloud-independent in v1.
- [ ] Event/status envelopes are compatible with future store-and-forward sync.

## H. Verification

- [ ] `cargo check` passes from clean state.
- [ ] Targeted tests pass for all touched slices.
- [ ] Architectural and resilience assertions are documented with evidence.
- [ ] Unit tests cover domain and policy behavior touched by the release.
- [ ] Integration tests cover adapter/persistence/runtime compositions touched by the release.
- [ ] Fault-injection tests validate recoverable failures without routine manual restart.
- [ ] Contract tests validate extension compatibility promises.
- [ ] Soak or long-run stability evidence exists for runtime-critical changes.
- [ ] Code review confirms Clean Code, SOLID, DRY, and Rust best-practice compliance.

## I. Release readiness

- [ ] Operator can understand runtime behavior in one page.
- [ ] Remaining backlog is explicit and prioritized as post-v1.
- [ ] No hidden blockers remain for v1 claim.
- [ ] Extension contracts are documented for developers (how to build optional adapters safely).
- [ ] Test report is attached with pass/fail evidence per test layer.