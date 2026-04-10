# Sprint 1 Executable Plan (Slice 0)

Date: 2026-04-10
Status: Ready

## Sprint goal

Lock architecture so implementation can move fast without coupling mistakes.

## Team model

Use a compact team with clear role ownership (can be multiple hats for one person):

1. Architecture Owner
   - owns boundaries, contracts, and dependency rules.
2. Runtime Reliability Owner
   - owns failure model, storage profile behavior, and resilience acceptance criteria.
3. Test Program Owner
   - owns test matrix, quality gates, and acceptance evidence.

## Backlog (single-track)

1. Define core/adapters/runtime dependency map and forbidden imports list.
2. Define `ProtocolCapability` contract for LoRaWAN policy modules.
3. Define install-time profile schema (`constrained`, `balanced`, `high-capacity`).
4. Define extension contract compatibility policy (hybrid SemVer window).
5. Define test matrix by slice with mandatory evidence gates.
6. Prepare Slice 1 kickoff checklist (implementation-ready).

## Definition of done (Sprint 1)

1. All backlog items merged into canonical docs.
2. Every item has explicit acceptance criteria.
3. Critical KPI pair is mapped to measurable checks:
   - edge reliability,
   - architecture integrity.
4. Slice 1 can start without unresolved architecture decisions.

## Risks and mitigations

1. Risk: scope creep into implementation.
   - Mitigation: no coding beyond Slice 0 deliverables.
2. Risk: ambiguous contracts.
   - Mitigation: every contract includes examples and non-goals.
3. Risk: tests deferred.
   - Mitigation: test matrix is mandatory Sprint 1 output.

## Weekly cadence (within sprint)

1. Day 1 planning: lock sprint backlog and acceptance.
2. Daily async updates: progress, blocker, KPI risk.
3. Mid-sprint review: architecture and test-gate sanity check.
4. End-of-sprint review: acceptance evidence and Slice 1 go/no-go.
