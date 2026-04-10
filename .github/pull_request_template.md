## What & Why

<!-- One paragraph. What changed? Why now? Link to issue or ADR if applicable. -->

Closes #<!-- issue number, or N/A -->

## Type of Change

- [ ] Bug fix (non-breaking)
- [ ] Feature (non-breaking)
- [ ] Refactor (behavior-preserving)
- [ ] Stability / resilience improvement
- [ ] Performance optimization
- [ ] Docs / ADR
- [ ] Test-only
- [ ] Breaking change — describe impact below

## Stability & Hardware Impact

<!-- Skip if docs/test-only PR. Required for all code changes. -->

- [ ] UDP ingester critical path: not affected / intentionally changed (describe)
- [ ] Memory: bounded allocations confirmed (no unbounded Vec/HashMap growth)
- [ ] RPi 3 (1 GB RAM, ARM v7): tested or analyzed as compatible
- [ ] Circuit breaker: new DB/external calls go through circuit breaker

## Verification

```bash
cargo check
cargo test --all
cargo clippy -- -D warnings
```

<!-- Add any additional commands or manual steps used to verify. -->

## Checklist

- [ ] ADR created or referenced if this is an architectural decision (`docs/adr/`)
- [ ] Tests cover the new behavior (not just the happy path)
- [ ] Docs updated if behavior or API surface changed
- [ ] Change is scoped to the PR's stated goal (no unrelated edits)
