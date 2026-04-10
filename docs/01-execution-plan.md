# Maverick Execution Plan (Runtime-First)

Date: 2026-04-10
Status: Active

Execution governance reference: `docs/03-operating-model.md`.

## Strategy

1. Build vertical slices with explicit evidence.
2. Lock architecture before feature expansion.
3. Reject hidden coupling and hidden runtime dependencies.
4. Optimize for edge reliability over feature breadth.
5. Execute with strict single-track WIP limit (one active slice at a time).

## Sprint 1 (achievable baseline)

Duration: 2 weeks
Active slice: Slice 0 only
Method: lightweight single-track Scrumban (`docs/03-operating-model.md`)

Sprint objective:
Freeze architecture and runtime contracts with testable acceptance gates, so coding in Slice 1 does not drift.

Sprint deliverables:

1. Architecture boundary map (core/adapters/runtime) with dependency rules.
2. Protocol capability module contract (v1 module + growth path).
3. Install-time profile contract and default profile matrix.
4. Extension contract policy (hybrid SemVer window) documented as enforceable rule.
5. Test strategy baseline mapped to upcoming slices (unit/integration/contract/fault/soak).

Sprint acceptance:

1. Canonical docs fully aligned and conflict-free.
2. Slice 0 entry/exit gates explicitly documented and reviewable.
3. First implementation backlog for Slice 1 is created with test-first scope.
4. No open architecture ambiguity remains for v1 execution start.

Sprint anti-goals:

1. No protocol feature implementation yet.
2. No sync runtime implementation.
3. No parallel second slice.

## Focus governance (anti-scope-drift)

### Execution mode

1. WIP limit = 1 active slice.
2. `Now` includes only the current active slice.
3. `Next` and `Later` are frozen backlog, not parallel work.

### Critical KPIs (must not regress)

1. Edge reliability: no routine manual restart under recoverable fault scenarios.
2. Architectural integrity: no forbidden dependency/coupling across core boundaries.

### Stop-the-line rule

If either critical KPI regresses, all feature work pauses until recovery is verified.

### Slice entry/exit discipline

1. A slice cannot start without explicit entry criteria and tests to prove the target behavior.
2. A slice cannot close without passing evidence for reliability and boundary integrity.
3. No carry-over "we fix later" items inside the same slice.

## Slice 0 - Architecture lock (mandatory)

Objective:
Freeze architecture contracts and quality rules before implementation growth.

Deliverables:
1. Core/adapters/runtime boundary map.
2. Port contracts (`traits`) for transport, persistence, audit, extension outputs.
3. Error taxonomy and degradation state machine.
4. Dependency rules document for crates/modules.
5. Extension contract versioning rules and compatibility policy.
6. Protocol capability-module strategy document (v1 module + v1.x growth path).
7. Install-time operational profile contract.

Done when:
1. Core can compile without transport/API/database concrete crates.
2. Boundary rules are documented and testable.

## Slice 1 - Core minimum complete LNS

Objective:
Deliver core LoRaWAN v1.0.x Class A logic as offline-first runtime behavior.

Deliverables:
1. Session and uplink/downlink core use cases.
2. Region and band-plan policy baseline for v1 regions.
3. Typed domain errors and invariants.
4. First protocol capability module: LoRaWAN 1.0.x Class A.

Done when:
1. Core logic is framework-independent.
2. Policy tests cover accepted/rejected protocol scenarios.
3. Protocol/version expansion can be added by new modules without changing core orchestration.

## Slice 2 - Durable local persistence + bounded retention

Objective:
Guarantee local truth and continuity under constrained storage.

Deliverables:
1. Durable persistence adapter default.
2. Hybrid retention policy (tiered + circular under pressure).
3. Recovery flow after restart/power loss.
4. Install-time profile presets for constrained, balanced, and high-capacity devices.

Done when:
1. Critical write path is durable.
2. Hard-limit behavior keeps node running with observable data rollover.
3. Install-time selected profile deterministically drives runtime storage behavior.

## Slice 3 - Transport and adapter isolation

Objective:
Keep radio and management adapters outside core while preserving runtime stability.

Deliverables:
1. First transport adapter implementation behind core transport contract.
2. Optional management adapter surface.
3. Isolation rules: bounded queues, timeout, retry/backoff, circuit break at I/O boundary.
4. Adapter contract examples for future integrations (MQTT/cloud connectors) without runtime plugin loading.

Done when:
1. Adapter failure does not crash core loop.
2. Core keeps processing while non-critical adapter paths degrade.

## Slice 4 - Health and operational visibility

Objective:
Make degradation obvious and actionable in edge environments.

Deliverables:
1. Internal health state model (`Healthy`, `Degraded`, `Unhealthy`).
2. Minimal metrics for pressure/failure paths.
3. Runtime runbook for field operations.
4. Mandatory local CLI visibility surface (`status`, `health`, `recent-errors`).
5. Mandatory local diagnostics journal with bounded retention and structured records.
6. Optional exportable node diagnostics snapshot for support cases.

Done when:
1. Operators can detect and classify pressure before manual restart is needed.
2. Health transitions are test-covered.
3. A field operator can inspect node state and recent failures locally without any external observability stack.
4. Snapshot export remains optional and does not add runtime burden when disabled.

## Slice 5 - Sync-ready contracts (no runtime sync in v1)

Objective:
Prepare for future cloud sync without introducing v1 runtime dependency.

Deliverables:
1. Extension contracts for future outbox/checkpoint integration.
2. Event/audit contract stability for v1.x sync work.
3. Store-and-forward-friendly status/event envelope format for intermittent network sync later.

Done when:
1. V1 is cloud-independent in runtime.
2. Future sync can be added without redesigning core.

## Cross-slice quality gates

Each slice requires:

1. `cargo check` for workspace.
2. Unit and integration tests for touched behavior.
3. No boundary violations in dependencies.
4. Explicit failure-mode tests for any runtime-critical path touched.
5. Extension-contract compatibility checks when public adapter ports change.
6. Evidence of test updates when behavior changes (no behavior change without tests).

## Test program (critical path)

Every active slice must map to a test plan before coding:

1. unit test scope,
2. integration test scope,
3. resilience/fault-injection scope,
4. performance or soak scope when runtime behavior is affected.

Mandatory gates before slice acceptance:

1. protocol invariants pass,
2. boundary integrity checks pass,
3. recoverable fault scenarios pass without requiring manual restart,
4. no untested critical-path changes.

## Definition of complete (v1)

1. Slices 0-5 accepted.
2. Runtime behavior matches `docs/00-product-intent.md`.
3. Documentation and runbook reflect actual runtime behavior.
4. Remaining work is explicit and treated as post-v1 scope.