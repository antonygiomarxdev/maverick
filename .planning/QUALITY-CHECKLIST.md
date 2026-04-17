# Quality Checklist

Before closing each phase, verify:

## ⚠️ MANDATORY: Always Ask First

**Before writing ANY code, ALWAYS ask:**
- Is this Clean Code? (single responsibility, clear naming, no duplication)
- Is this SOLID? (does it follow single responsibility, open/closed, Liskov, interface segregation, dependency inversion)
- Is this KISS? (simple and readable over clever)
- Does this follow Hexagonal Architecture? (ports/adapters, dependencies point inward)
- Is there magic values? (extract to constants)
- Is this file too large? (> 400 lines = refactor candidate)

**If ANY of these is unclear — STOP and refactor before proceeding.**

## Clean Code
- [ ] Read: `.cursor/rules/rust-clean-code.mdc`
- [ ] No magic values — see `.cursor/rules/rust-no-magic-values.mdc`
- [ ] Follows Rust idioms — see `.cursor/skills/rust-best-practices/SKILL.md`
- [ ] Functions have single responsibility
- [ ] No duplication across modules
- [ ] File size < 400 lines (refactor if larger)

## Architecture
- [ ] Hexagonal architecture — ports/adapters separation — see `.cursor/rules/rust-solid-hexagonal.mdc`
- [ ] Single responsibility per module
- [ ] Dependencies point inward (domain → application → adapters)
- [ ] No circular dependencies

## Code Quality
- [ ] `cargo fmt` passes
- [ ] `cargo clippy` passes (no warnings)
- [ ] Unit tests for core logic
- [ ] Integration tests for adapter boundaries

## Vision Check
- [ ] This phase brings Maverick closer to "offline-first, self-contained LoRaWAN stack"?
- [ ] Nothing in this phase contradicts the vision?
- [ ] Extensions remain optional and isolated from core?
- [ ] No cloud dependencies added to core runtime?

## References
- `.cursor/rules/rust-clean-code.mdc`
- `.cursor/rules/rust-no-magic-values.mdc`
- `.cursor/rules/rust-solid-hexagonal.mdc`
- `.cursor/rules/rust-linter-configuration.mdc`
- `.cursor/skills/rust-best-practices/SKILL.md`
- `AGENTS.md` (Rust engineering standards)
