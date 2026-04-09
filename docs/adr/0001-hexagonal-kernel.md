# ADR-0001: Hexagonal Kernel, Local-First Runtime

## Status

Accepted

## Context

Maverick must grow into a production-grade frontier LNS without coupling core logic to HTTP, SQL statements, gateway hardware vendors, or optional cloud integrations.

## Decision

The project uses a hexagonal architecture with these boundaries:

- domain types and entities remain infrastructure-agnostic
- application use cases depend on ports, not transport or persistence details
- adapters implement transport, persistence, and integration concerns
- optional integrations remain outside the kernel critical path

The runtime is local-first. Cloud connectivity is additive, not foundational.

## Consequences

- refactors happen early to preserve a small clean kernel
- repository ports are preferred over direct SQL usage in application logic
- semantic events and auditability become first-class runtime concerns
- the project remains portable across hardware and deployment profiles