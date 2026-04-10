# Technical review checklist (sprint / PR gate)

Use this alongside [`AGENTS.md`](../AGENTS.md) so **clean code**, **SOLID / hexagonal boundaries**, and **no magic values** are explicit review criteria—not optional polish.

## 1. Clean code (readability & maintainability)

- [ ] Functions and modules have a single clear responsibility; file size stays within the pragmatic policy in the delivery plan (domain/core modules aim ≤ ~250 LOC; > ~400 LOC needs justification).
- [ ] Error paths are handled; no silent `unwrap`/`expect` on external I/O or user input.
- [ ] Naming expresses intent; comments explain *why*, not *what*.
- [ ] Skill reference: [`.cursor/skills/rust-clean-code/SKILL.md`](../.cursor/skills/rust-clean-code/SKILL.md).

## 2. SOLID & hexagonal architecture

- [ ] `maverick-core` depends on ports (traits) and domain types only—no SQLite, UDP, HTTP, or other infrastructure crates.
- [ ] Adapters implement ports; composition (e.g. `maverick-runtime-edge`) wires concrete types—no “reach through” from domain to adapters.
- [ ] Transport and persistence failures surface as `AppResult` / health detail, not panics, for operator-visible degradation.
- [ ] Skill reference: [`.cursor/skills/rust-solid-hexagonal/SKILL.md`](../.cursor/skills/rust-solid-hexagonal/SKILL.md).

## 3. No magic values (literals)

- [ ] Repeated semantic strings (JSON keys, operation labels, CLI messages) live in `const`, enums, or centralized modules—not copy-pasted.
- [ ] Policy numbers (timeouts, retries, caps, ratios) are named and documented; SQL identifiers stay centralized in adapter schema modules.
- [ ] Skill reference: [`.cursor/skills/rust-no-magic-values/SKILL.md`](../.cursor/skills/rust-no-magic-values/SKILL.md).

## 4. Verification

- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] `cargo test --workspace` (or documented subset if platform blocks integration binaries)
- [ ] New I/O boundaries have fault-oriented tests (timeout, busy, circuit open) where applicable.

## 5. Documentation & evidence

- [ ] [`docs/05-test-program.md`](05-test-program.md) updated if test layers changed.
- [ ] Slice evidence doc updated for material behavior or residual risk ([`docs/slice-3-evidence.md`](slice-3-evidence.md)).
