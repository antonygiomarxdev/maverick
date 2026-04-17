# Phase 5: TUI Device Management - Context

**Gathered:** 2026-04-16
**Status:** Ready for planning
**Mode:** Auto-generated (discuss skipped via autonomous workflow)

<domain>
## Phase Boundary

An operator can add, inspect, and remove LoRaWAN devices entirely through the terminal UI (TUI). No manual TOML editing required. Hardware probe results visible in TUI. Legacy lns-config.toml import supported.

</domain>

<decisions>
## Implementation Decisions

### Claude's Discretion
All implementation choices are at Claude's discretion. Use ROADMAP phase goal, success criteria, and codebase conventions to guide decisions.

### Key Technical Decisions Needed
- **TUI framework**: Follow existing TUI patterns in maverick-extension-tui
- **IPC mechanism**: How TUI communicates with maverick-edge (stdin/stdout, Unix socket, HTTP)
- **Device CRUD operations**: Add, list, remove, promote operations

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/maverick-extension-tui/src/`: Existing TUI extension
- `crates/maverick-extension-tui/src/lns_wizard.rs`: Device provisioning wizard
- `crates/maverick-extension-tui/src/menu_lorawan.rs`: LoRaWAN menu
- `crates/maverick-domain/src/identifiers.rs`: DevEUI, DevAddr types

### Established Patterns
- TTY menu system with arrow key navigation
- Shell out to `maverick-edge` CLI for operations
- IPC via stdin/stdout JSON commands

### Integration Points
- maverick-edge CLI for device operations
- SQLite via existing persistence adapter
- lns-config.toml parser for DEV-04

</code_context>

<specifics>
## Specific Ideas

Requirements: DEV-01, DEV-02, DEV-03, DEV-04, DEV-05, CORE-03

</specifics>

<deferred>
## Deferred Ideas

None.

</deferred>
