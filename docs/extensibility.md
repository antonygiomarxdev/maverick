# Extensibility Model

Maverick is designed as a production-ready kernel with clean optional layers on top.

## Extension Surfaces

- Structured events from the internal event bus
- Audit records that capture decisions and outcomes
- Stable administrative APIs
- Repository-independent application use cases
- Configuration switches for optional adapters

## Rules for Extensions

- An extension must not become required for the offline critical path.
- An extension must consume contracts rather than internal implementation details.
- An extension should be disposable without destabilizing the kernel.

## Examples

- Publish verified uplinks to MQTT
- Push device state into AWS IoT Core
- Run anomaly detection from external AI tooling
- Export audit streams into observability stacks