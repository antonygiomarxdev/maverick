# Maverick Operating Model (Focus and Agile Execution)

Date: 2026-04-10
Status: Active

## Purpose

This document defines how the team executes v1 without losing focus.
If process discussions conflict with this document, this document wins.

## Mission for v1

Build a runtime-first, offline-first edge LNS that:

1. keeps operating in intermittent or no-network environments,
2. avoids routine manual restarts,
3. stays decoupled and extensible for future protocol and integration growth.

## Execution mode (mandatory)

1. Single-track execution only (`WIP = 1`).
2. Only one active slice at a time.
3. `Now` is the only area where coding work is allowed.
4. `Next` and `Later` stay frozen backlog until `Now` is accepted.

## Critical KPIs (stop-the-line)

Two KPIs are critical and non-negotiable:

1. Edge reliability:
   - recoverable faults must not require manual restart as normal behavior.
2. Architecture integrity:
   - no forbidden coupling between core and infrastructure.

If either KPI regresses, all feature work stops until recovered and verified.

## Testing policy (critical)

Testing is first-class work, not follow-up work.

1. Every slice starts with a written test plan.
2. No implementation task is considered done without corresponding test evidence.
3. Critical runtime paths require fault-injection coverage.
4. Runtime-critical changes require long-run stability validation.
5. If tests are missing for a critical path, release is blocked.

## Code quality standard (non-negotiable)

All implementation work must follow:

1. Clean Code (readable naming, small focused units, explicit error flows).
2. Clean Architecture (strict boundary separation and dependency direction).
3. SOLID principles (especially SRP, ISP, and DIP in module/service design).
4. DRY (remove duplication without over-abstraction).
5. Rust best practices (strong typing, exhaustive handling, explicit ownership/lifetimes where relevant, safe concurrency).

If a change passes tests but violates these standards in critical paths, it is not accepted.

## Agile methodology for this project

The team uses a lightweight single-track Scrumban model:

1. Weekly planning (choose one slice/sub-slice for `Now`).
2. Daily async check-in (progress, blocker, KPI risk).
3. Mid-week technical review (boundary and reliability risk check).
4. End-of-week acceptance review with evidence.
5. Retrospective focused on reliability and architecture debt.

## Slice governance

### Entry criteria (before starting a slice)

1. Problem statement and scope are explicit.
2. Acceptance criteria are testable.
3. Boundary impact is documented.
4. Failure modes for touched runtime paths are listed.

### Exit criteria (before closing a slice)

1. `cargo check` and targeted tests pass.
2. Reliability tests for touched paths pass.
3. No new boundary violations.
4. Operator and developer docs updated if behavior changed.
5. Residual risks are explicit and assigned.
6. Test evidence includes at least unit + integration + fault scenarios for touched critical paths.
7. Code review confirms Clean Code/Clean Architecture/SOLID/DRY/Rust-practice compliance.

## Minimal planning structure

Always keep this structure visible:

1. Now: one active slice.
2. Next: up to two queued slices.
3. Later: everything else.

No additional queue levels.
No hidden parallel tracks.

## Developer-friendly extensibility rules

1. Extension contracts are versioned and stable.
2. v1.x has no breaking contract changes.
3. Deprecations require a documented compatibility window.
4. Future adapters (HTTP, MQTT, AWS IoT, others) must not require core rewrites.

## Visibility baseline policy (v1)

Mandatory:

1. local CLI (`status`, `health`, `recent-errors`),
2. structured rotating local logs.

Optional:

1. exportable diagnostics snapshot on demand.

No heavy external observability platform is required for v1 operation.

## Decision log rule

Every architecture-impacting decision must be written down with:

1. context,
2. decision,
3. consequences,
4. rollback or migration note.

If not documented, it is not considered accepted.
