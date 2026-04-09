# UDP Ingester

The UDP ingester is the first radio-facing surface of the Maverick kernel.

Its job is to accept Semtech UDP `PUSH_DATA` datagrams, validate their transport boundary, transform them into typed radio observations, and persist them locally with structured auditability.

## Current MVP Scope

- Semtech UDP `PUSH_DATA`
- `rxpk` parsing
- payload base64 decode
- `datr` parsing into spreading factor and bandwidth
- gateway upsert
- uplink frame persistence
- semantic events and audit records for accepted or rejected packets

## Important Boundary

The current ingester persists a raw radio observation, not a fully interpreted LoRaWAN uplink.

That distinction matters because Semtech UDP delivers radio metadata and PHY payload bytes, but not a complete application-level interpretation of device identity, frame counters, or decoded payload semantics.

## Non-Goals for This MVP

- full LoRaWAN join handling
- full ACK/downlink semantics
- hardware-specific concentrator bindings
- cloud replication or external connectors