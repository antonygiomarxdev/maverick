# Maverick Product Intent (Runtime-First)

Date: 2026-04-10
Status: Active

## Canonical rule

This file is the source of truth for product scope and non-negotiables.
If any other document conflicts with this file, this file wins.

## Product definition

Maverick is an offline-first LoRaWAN edge runtime.
It is not a full OS image in v1.

The runtime must:

1. run locally on constrained hardware,
2. keep processing LoRa traffic without internet,
3. store operational truth locally in durable form,
4. avoid mandatory coupling to optional extensions.

## Non-negotiables

1. No hidden coupling from core to infrastructure details.
2. Core must keep working even if optional extensions fail.
3. No unbounded memory structures in critical paths.
4. Recoverable failures must never require manual restart as normal behavior.
5. Architecture and code quality are strict: clean architecture, SOLID, strong typing, explicit error handling, testability.
6. Rust best practices are mandatory, not optional.
7. Developer-friendly extensibility is mandatory from day one.
8. Runtime visibility of node state is mandatory from day one.
9. Test rigor is mandatory from day one across unit, integration, fault-injection, and long-run stability scenarios.

## V1 scope (locked)

### In scope

1. Single runtime binary/service focused on edge.
2. LoRaWAN 1.0.x Class A.
3. Region baseline: EU868, US915, AU915, AS923, EU433.
4. Local durable persistence as default behavior.
5. Bounded resilience model with backpressure and degradation states.
6. Optional adapters around core (no runtime plugin loading in v1).
7. Minimal local visibility surface for node operation and diagnostics.
8. Stable extension contracts for post-v1 integrations.

### Out of scope in v1

1. Runtime sync engine to cloud.
2. LoRaWAN Class B and Class C.
3. Runtime dynamic plugin framework.
4. Full parity with large cloud-first platforms.

## Storage policy (runtime behavior)

Storage policy is configurable.
Default behavior is hybrid:

1. tiered retention by data criticality,
2. circular behavior for continuity under pressure.

When storage reaches hard limit, runtime continues operating and can overwrite oldest records, including critical history, instead of stopping the node.
This must be observable via health and alerts.

## Architecture boundaries

### Core

1. Domain rules and entities.
2. Use cases and policies.
3. Port interfaces (traits) for persistence, transport, audit, and extension outputs.
4. Typed errors and state machine for health/degradation.

### Adapters

1. Transport adapters (radio ingress/egress).
2. Persistence adapters.
3. Management API adapter.
4. Observability/export adapters.
5. Integration adapters (MQTT, cloud connectors, vendor bridges) as optional layers.

### Runtime composition

1. Wiring and startup only.
2. No business logic in composition layer.

## Protocol evolution strategy (post-v1 growth without rewrites)

Maverick evolves by capability modules, not by core rewrites.

1. Keep a stable core protocol orchestration and policy interfaces.
2. Implement version/class behavior in policy modules behind contracts.
3. Start with LoRaWAN 1.0.x Class A module in v1.
4. Add Class B, Class C, and newer LoRaWAN versions as additional capability modules in v1.x+.

This model follows proven market direction where mature LNS keep region/protocol logic in explicit policy layers and avoid coupling transport/runtime wiring to protocol evolution.

## Installation profile model (v1)

Operational profile is selected during installation/setup.
This setup choice defines default storage/retention and resilience posture for the node.

In v1:

1. profile is decided at install time,
2. runtime behavior follows that profile deterministically,
3. remote dynamic profile switching is not required.

## Future sync direction (not v1 runtime)

V1 must expose contracts so sync can be added in v1.x without redesigning core.
Cloud sync is extension work and must not be required for local operation.

## Visibility architecture baseline (v1)

Visibility in v1 is required but lightweight.
No heavy external observability stack is required to run.

V1 runtime must expose:

1. health state machine (`Healthy`, `Degraded`, `Unhealthy`) per node,
2. bounded local diagnostics journal (structured events),
3. pressure indicators (storage, queue depth, retry/circuit states),
4. operator-readable local status snapshot via management surface or local CLI.

V1 mandatory visibility bundle:

1. local CLI (`status`, `health`, `recent-errors`),
2. local structured rotating logs.

V1 optional visibility bundle:

1. exportable diagnostic snapshot (on-demand support artifact).

This baseline is inspired by proven edge patterns:

1. service-level metrics and health endpoints used in LoRaWAN stacks,
2. store-and-forward operational visibility for intermittent connectivity environments.

## Developer-friendly extension model (v1 + v1.x)

### v1 behavior

1. No runtime plugin loading.
2. Extensions are optional adapters wired at build/deploy time.
3. Core emits stable typed events and consumes narrow port contracts.

### v1.x readiness

1. Keep extension API contract stable and versioned.
2. Allow future integration packages (HTTP, MQTT, AWS IoT, others) without core changes.
3. Treat every integration as replaceable and failure-isolated.

### Contract compatibility policy

Maverick extension contracts follow hybrid SemVer window policy:

1. no breaking changes in v1.x,
2. breaking changes only in next major,
3. deprecations must be documented and supported through a compatibility window before removal.

### DX requirements

1. Clear extension contract documentation.
2. Reference adapter templates.
3. Predictable configuration model by capability profile (small, medium, large hardware).
4. Backward compatibility policy for extension contracts.

## Definition of success

Maverick v1 is successful when:

1. edge runtime runs continuously without internet,
2. core behavior remains stable when optional adapters are disabled or degraded,
3. local state survives restarts and outages with deterministic recovery,
4. architectural boundaries are enforced by crate/module dependency rules,
5. resilience behavior is verified by tests and failure scenarios.
6. node visibility baseline works without external observability platform.
7. extension contracts are stable enough to implement first-party and third-party adapters without core rewrites.
8. test evidence includes protocol correctness, boundary integrity, resilience under faults, and soak behavior on constrained hardware profiles.

## Test strategy baseline (v1)

V1 requires multiple test layers:

1. unit tests for domain rules, protocol policies, and invariants,
2. integration tests for ports/adapters and persistence behavior,
3. contract tests for extension interfaces and compatibility guarantees,
4. fault-injection tests for DB lock, timeout, malformed traffic, burst pressure, and adapter degradation,
5. soak tests for long-running edge stability under realistic load.

No slice is considered complete without test evidence for the layers it touches.