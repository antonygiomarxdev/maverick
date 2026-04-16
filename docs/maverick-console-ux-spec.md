# Maverick Console UX Spec (Optional Extension)

Date: 2026-04-10  
Status: Draft for implementation

## Scope

This spec applies only when the optional console extension is installed.

Current binary implementation: `maverick-edge-tui`  
Product-facing name in UX: `maverick`

## UX goals

- Professional terminal experience with clear visual hierarchy.
- Fast operational workflows (status, health, ingest-loop).
- Discoverable keyboard-driven navigation (`?` help everywhere).

## Home screen

```text
+--------------------------------------------------------------+
| maverick                                       v0.1.x (beta) |
| Edge operations console                                      |
|--------------------------------------------------------------|
| 1. Runtime status                                             |
| 2. Health overview                                            |
| 3. Start ingest loop                                          |
| 4. Setup and config                                           |
| 5. Extensions                                                 |
|                                                              |
| [?] Help   [Q] Quit                                           |
+--------------------------------------------------------------+
```

## Health overview

```text
+--------------------------------------------------------------+
| Health overview                                               |
|--------------------------------------------------------------|
| Global: DEGRADED                                              |
|                                                              |
| Components:                                                   |
|  - storage     : healthy                                     |
|  - radio       : degraded (timeout spikes)                   |
|  - ingest_loop : stopped                                     |
|                                                              |
| [V] Details  [R] Refresh  [Esc] Back                          |
+--------------------------------------------------------------+
```

## Extensions view

```text
+--------------------------------------------------------------+
| Extensions                                                    |
|--------------------------------------------------------------|
| maverick console : installed                                 |
| http             : coming soon                               |
| mqtt             : coming soon                               |
|                                                              |
| [Enter] Details  [Esc] Back                                   |
+--------------------------------------------------------------+
```

## Keyboard contract

- `Enter`: select/confirm
- `Esc`: back
- `Q`: quit
- `?`: contextual help
- `R`: refresh/retry

## Help panel requirements

Every screen help (`?`) must show:

- screen purpose,
- available keys,
- one practical command example.

## Error UX rules

- No stack traces in operator-facing views.
- Always show what failed + next action.

Template:

```text
[ERROR] <action> failed
Why: <short reason>
Next: <single actionable command>
```

## Command mapping

- Runtime status -> `maverick-edge status`
- Health overview -> `maverick-edge health`
- Start ingest loop -> `maverick-edge radio ingest-loop ...`
- Extensions data -> setup config + available binaries

No menu option may be shown if its command path is not functional.

## Acceptance criteria (console)

- `?` help available on all primary screens.
- Navigation and action keys are consistent across screens.
- User can execute day-2 core actions without leaving the console.
