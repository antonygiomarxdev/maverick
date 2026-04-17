# Quality Checklist

Before closing each phase, verify:

## Clean Code
- [ ] Read: `.cursor/rules/rust-clean-code.mdc`
- [ ] No magic values — see `.cursor/rules/rust-no-magic-values.mdc`
- [ ] Follows Rust idioms — see `.cursor/skills/rust-best-practices/SKILL.md`

## Architecture
- [ ] Hexagonal architecture — ports/adapters separation — see `.cursor/rules/rust-solid-hexagonal.mdc`
- [ ] Single responsibility per module
- [ ] Dependencies point inward (domain → application → adapters)

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
- `AGENTS.md` (Rust engineering standards)
