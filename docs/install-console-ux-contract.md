# Install and Console UX Contract

Date: 2026-04-10  
Status: Active (index document)

## Why this split exists

To avoid confusion, Maverick UX is split into two independent surfaces:

1. **Core first-run onboarding** (mandatory)
2. **Maverick console extension** (optional)

This file is only a contract/index. Detailed specs live in separate files.

## Contract (non-negotiable)

- Installation is successful when **core first-run onboarding** completes.
- Extension selection never blocks core setup completion.
- If no extension is selected, Maverick remains fully operable through `maverick-edge`.
- Console UX must enhance day-2 operations but must not own first-run success criteria.

## Detailed specs

- Core onboarding spec: [`install-onboarding-ux-spec.md`](install-onboarding-ux-spec.md)
- Optional console spec: [`maverick-console-ux-spec.md`](maverick-console-ux-spec.md)

## Shared UX rules

- Keep language operator-first, concise, and actionable.
- Use consistent semantic prefixes: `[OK]`, `[INFO]`, `[WARN]`, `[ERROR]`.
- Never expose visible actions without a working command path.
