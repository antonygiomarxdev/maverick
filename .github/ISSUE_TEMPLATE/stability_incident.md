---
name: Stability incident
about: Report an unplanned restart, hang, or crash on edge hardware
labels: stability, high-priority
---

## What Happened

<!-- Describe the incident. When did it start? When did it end? Was it resolved manually? -->

## Hardware & Deployment

- Hardware: <!-- RPi 3 / RPi 4 / embedded ARM / other -->
- Deployment profile: <!-- edge / gateway / server -->
- Uptime before incident:
- How resolved: <!-- manual restart / auto-recovered / still ongoing -->

## Symptoms Observed

- [ ] Process exited (crash)
- [ ] UDP ingester stopped receiving (radio offline)
- [ ] High memory (>800 MB on RPi 3)
- [ ] High CPU (>80% sustained)
- [ ] Database locked / slow queries
- [ ] Silent data loss (frames missing in audit log)
- [ ] Other:

## Logs

<!-- Paste relevant structured logs. Include timestamps. Even partial logs help. -->

```
```

## `/api/v1/health` Response at Time of Incident

<!-- Run: curl http://localhost:8080/api/v1/health -->

```json
```

## Impact

- Downtime duration:
- Frames lost (estimated):
- Devices affected:

## What Was Happening Before

<!-- Any unusual load, device floods, network issues, or configuration changes? -->
