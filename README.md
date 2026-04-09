# Maverick

Maverick is a local-first LoRaWAN Network Server kernel designed for two realities at once:

- low-resource gateways close to the field
- high-capacity servers in central infrastructure

The core idea is simple: keep one stable kernel, then scale behavior with runtime profiles, not with divergent codebases.

## Why Maverick

Maverick focuses on operational resilience and clean boundaries:

- offline-first execution
- single binary runtime
- typed application contracts and hexagonal architecture
- semantic events and audit records as first-class outputs
- optional integrations built around a stable kernel boundary

## What Works Today

Current implemented capabilities:

- HTTP management API for device lifecycle
- gateway listing and healthy gateway discovery API
- downlink enqueue, listing and status retrieval API
- Semtech UDP ingest path for radio observations
- local persistence with SQLite schema bootstrap
- storage profile selection based on runtime constraints
- retention and buffering strategies for constrained hardware
- structured audit/event pipeline across API and UDP flows

## Quick Start

Run locally with Cargo:

```bash
cargo run -p maverick-core
```

Health check:

```bash
curl http://localhost:8080/api/v1/health
```

Create a device:

```bash
curl -X POST http://localhost:8080/api/v1/devices \
	-H "Content-Type: application/json" \
	-d '{
		"dev_eui": "0102030405060708",
		"app_eui": "0807060504030201",
		"app_key": "AQEBAQEBAQEBAQEBAQEBAQ==",
		"nwk_key": "AgICAgICAgICAgICAgICAg==",
		"class": "ClassA"
	}'
```

Enqueue a downlink:

```bash
curl -X POST http://localhost:8080/api/v1/devices/0102030405060708/downlinks \
	-H "Content-Type: application/json" \
	-d '{
		"gateway_eui": "AABBCCDDEEFF0011",
		"payload": "AQI=",
		"f_port": 10,
		"frequency_hz": 868100000,
		"spreading_factor": 7,
		"frame_counter": 1,
		"priority": "Normal"
	}'
```

Enqueue a downlink with automatic gateway selection:

```bash
curl -X POST http://localhost:8080/api/v1/devices/0102030405060708/downlinks \
	-H "Content-Type: application/json" \
	-d '{
		"payload": "AQI=",
		"f_port": 10,
		"frequency_hz": 868100000,
		"spreading_factor": 7,
		"frame_counter": 2,
		"priority": "High"
	}'
```

## API Surface (Current)

Health:

- GET /api/v1/health

Gateways:

- GET /api/v1/gateways
- GET /api/v1/gateways?status=Online|Offline|Timeout
- GET /api/v1/gateways/healthy

Devices:

- POST /api/v1/devices
- GET /api/v1/devices/:dev_eui
- PATCH /api/v1/devices/:dev_eui
- DELETE /api/v1/devices/:dev_eui

Downlinks:

- POST /api/v1/devices/:dev_eui/downlinks
- GET /api/v1/devices/:dev_eui/downlinks
- GET /api/v1/devices/:dev_eui/downlinks/:downlink_id

Boundary encoding rules:

- DevEUI and AppEUI as hex strings
- GatewayEUI as hex string when explicitly provided
- AppKey and NwkKey as base64 strings
- domain validation mapped to classified HTTP responses

## Deployment Profiles

Use the same image with different env presets:

- deploy/profiles/edge.env
- deploy/profiles/gateway.env
- deploy/profiles/server.env

Run with Docker Compose:

```bash
docker compose --env-file deploy/profiles/gateway.env up -d
docker compose --env-file deploy/profiles/edge.env up -d
docker compose --env-file deploy/profiles/server.env up -d
```

Or via helper script:

```bash
./scripts/run-profile.sh edge up
./scripts/run-profile.sh gateway logs
./scripts/run-profile.sh server restart
```

Profile intent:

- edge: minimal footprint (MAVERICK_STORAGE_PROFILE=extreme)
- gateway: balanced defaults (MAVERICK_STORAGE_PROFILE=auto)
- server: higher throughput posture (MAVERICK_STORAGE_PROFILE=high)

## Install Options

One-line installer on Linux hosts:

```bash
curl -sSf https://raw.githubusercontent.com/antonygiomarxdev/maverick/main/scripts/install.sh | sh
```

Container deploy:

- Dockerfile
- docker-compose.yml
- multi-arch release workflow at .github/workflows/release.yml

## Architecture and Design Docs

- docs/vision.md
- docs/kernel-boundary.md
- docs/extensibility.md
- docs/ai-readiness.md
- docs/frontier-runtime.md
- docs/management-api.md
- docs/udp-ingester.md
- docs/adr/0001-hexagonal-kernel.md

## Open Source Governance

- LICENSE
- CONTRIBUTING.md
- CODE_OF_CONDUCT.md
- SECURITY.md
- CHANGELOG.md

## Near-Term Roadmap

Priority items in progress:

- production-grade downlink sender integration with real gateway path
- gateway selection/failover policy for delivery
- API auth and operator-facing hardening
- release/process polish for external contributors