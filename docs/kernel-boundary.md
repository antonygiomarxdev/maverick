# Kernel Boundary

This document defines what belongs to the Maverick core kernel and what should remain optional.

## Core Kernel Responsibilities

- LoRaWAN ingress and essential protocol handling
- Local-first persistence and recovery behavior
- Cryptographic validation and trustworthy packet processing
- Stable application use cases and repository ports
- Structured observability, audit records, and classified errors
- Production-ready runtime defaults for edge deployment

## Optional Extension Responsibilities

- MQTT bridges
- AWS IoT Core adapters
- Webhooks and external notification systems
- Dashboards and advanced control planes
- AI orchestration layers or policy engines
- Cloud connectors and operator-specific workflows

## Boundary Rule

If the feature is useful but not required for the kernel to operate offline on the edge, it should start life as an extension.