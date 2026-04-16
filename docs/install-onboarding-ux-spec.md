# Install Onboarding UX Spec (Core Path)

Date: 2026-04-10  
Status: Draft for implementation

## Scope

This spec covers only the mandatory first-run onboarding path executed from installer/CLI.

## Flow

```text
Install Script -> Preflight -> Install Binaries -> Setup Wizard (4 steps)
             -> Extension Selection -> Final Summary -> Finish
```

## Wizard steps

1. Data directory (`MAVERICK_DATA_DIR`)
2. Gateway runtime defaults (`MAVERICK_GWMP_BIND`, timeouts)
3. Smoke checks (`maverick-edge --help`, `status`, `health`)
4. Extension selection (non-blocking for core completion)

## Core UX requirements

- Progress indicator required (`Step N/M`)
- Defaults shown before any input
- Inline validation feedback on every editable value
- Clear final summary with exact next commands
- Help discoverability with `?`

## ASCII views (core)

## Preflight

```text
+--------------------------------------------------------------+
| Maverick Installer                                            |
|--------------------------------------------------------------|
| [OK] Linux detected                                           |
| [OK] Architecture detected                                    |
| [OK] Install dir ready                                        |
| [OK] Release resolved                                          |
|                                                              |
| (1) Upgrade in place  (2) Reinstall clean  (3) Exit         |
+--------------------------------------------------------------+
```

## Step shell

```text
+--------------------------------------------------------------+
| Maverick First-Run Setup                         Step 2 of 4 |
|--------------------------------------------------------------|
| <step content>                                               |
| [Enter] Continue  [B] Back  [Q] Quit  [?] Help              |
+--------------------------------------------------------------+
```

## Data directory

```text
+--------------------------------------------------------------+
| Step 1/4 - Data storage                                      |
|--------------------------------------------------------------|
| Data directory: [/var/lib/maverick]                         |
| [OK] writable by runtime user                               |
| [Enter] Accept default  [E] Edit path                        |
+--------------------------------------------------------------+
```

## Gateway runtime

```text
+--------------------------------------------------------------+
| Step 2/4 - Gateway runtime                                   |
|--------------------------------------------------------------|
| GWMP bind: [0.0.0.0:17000]                                  |
| Read timeout (ms): [1000]                                   |
| Max messages: [1000]                                        |
| [T] Test bind now                                            |
+--------------------------------------------------------------+
```

## Smoke checks

```text
+--------------------------------------------------------------+
| Step 3/4 - Health checks                                     |
|--------------------------------------------------------------|
| [OK] maverick-edge --help                                   |
| [OK] maverick-edge status                                   |
| [OK] maverick-edge health                                   |
| [R] Re-run checks                                             |
+--------------------------------------------------------------+
```

## Extensions step (core perspective)

```text
+--------------------------------------------------------------+
| Step 4/4 - Extensions                                        |
|--------------------------------------------------------------|
| [x] maverick (console) - available                           |
| [ ] http - coming soon                                       |
| [ ] mqtt - coming soon                                       |
+--------------------------------------------------------------+
```

Rule: user may skip all extensions and still finish successfully.

## Final summary

```text
+--------------------------------------------------------------+
| Setup complete                                               |
|--------------------------------------------------------------|
| Installed: maverick-edge                                     |
| Config saved: /etc/maverick/setup.json                       |
| Next:                                                        |
|  1) maverick-edge status                                     |
|  2) maverick-edge health                                     |
|  3) maverick-edge radio ingest-loop ...                      |
| [F] Finish                                                   |
+--------------------------------------------------------------+
```

## Error message standard (core)

```text
[ERROR] Failed to bind 0.0.0.0:17000 (address in use)
Try: sudo ss -lunp | rg 17000
Then choose a different bind address in Step 2.
```

## Acceptance criteria (core onboarding)

- First-run onboarding completes with zero extensions selected.
- Every visible step action maps to a working command.
- Errors provide a specific next action.
