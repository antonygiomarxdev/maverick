# Phase 03: Protocol Security - Context

**Gathered:** 2026-04-16
**Status:** Ready for planning

<domain>

## Phase Boundary

Implement LoRaWAN protocol security: MIC (Message Integrity Code) verification and FCnt (Frame Counter) 32-bit support. This phase validates that every uplink is authentic before accepting it, and supports the full 32-bit frame counter to prevent sessions from breaking after 65535 uplinks.

</domain>

<decisions>

## Implementation Decisions

### MIC Verification Location
- **D-01:** MIC verification occurs in the **core layer** (after adapter parsing), not in the adapter itself. The adapter produces a parsed `Frame`, the core handles MIC using `SessionSnapshot.NwkSKey`.

### FCnt Rollover Detection
- **D-02:** FCnt rollover detected by **gap detection**: when `newFCnt < lastFCnt` and difference is large (> 1000), assume rollover and increment the high 16 bits of the local 32-bit counter.

### MIC Failure Mode
- **D-03:** When MIC fails: **reject with warning + metric**. Log at WARN level, increment security metric counter, do not store frame. This allows monitoring of potential attacks without spamming logs.

### Testing Strategy
- **D-04:** MIC testing via **unit tests with hardcoded frames from LoRaWAN spec test vectors**. Use known-good frames with known MIC values. Do not require real hardware for testing.

### Scope: What's In
- MIC verification (all uplinks validated before acceptance)
- FCnt 32-bit support (full 32-bit frame counter)
- FRMPayload decryption (AES-128 in end-device mode)
- Basic security metrics (MIC failures counter)

### Scope: What's Out
- Cloud-side security (future Maverick Cloud phase)
- OTAA join security (deferred to v2)
- Deep packet inspection (not in scope)

</decisions>

<canonical_refs>

## Canonical References

**Downstream agents MUST read these before planning or implementing.**

- `VISION.md` — Project vision: "LoRaWAN. Offline. Always."
- `ROADMAP.md` — Phase 03 goals and exit criteria
- `.planning/QUALITY-CHECKLIST.md` — Quality standards (clean code, SOLID, KISS, hexagonal)
- `.cursor/rules/rust-clean-code.mdc` — Clean code rules
- `.cursor/rules/rust-solid-hexagonal.mdc` — Hexagonal architecture rules

</canonical_refs>

<code_context>

## Existing Code Insights

### Reusable Assets
- `SessionSnapshot` already contains `NwkSKey` — needed for MIC calculation
- `UplinkSource` port trait already defined — MIC verification happens after this port
- Existing `IngestUplink` use case — will need to integrate MIC verification

### Integration Points
- MIC verification happens between `UplinkHandler` port receipt and SQLite persistence
- FCnt state stored in session (needs migration from 16-bit to 32-bit)
- Security metrics can use existing metrics infrastructure (if any)

### Established Patterns
- Error types should use `thiserror`
- No `unwrap()` in production code
- Hexagonal: adapter → port → core → port → adapter

</code_context>

<specifics>

## Specific Requirements

- MIC calculation per LoRaWAN 1.0.x spec (B0 block for uplinks)
- AES-128 decryption with NwkSKey for FRMPayload
- Session state must persist FCnt as 32-bit value
- Security events logged for audit trail

</specifics>

<deferred>

## Deferred Ideas

- **Observability phase (Phase 07):** Security event logging in detail will be handled there
- **Extension IPC (Phase 05):** Security metrics exposure to extensions deferred

</deferred>

---

*Phase: 03-protocol-security*
*Context gathered: 2026-04-16*
