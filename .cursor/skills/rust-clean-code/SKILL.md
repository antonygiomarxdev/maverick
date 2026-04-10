---
name: rust-clean-code
description: >-
  Clean-code guidance for Rust: function size, modules, error handling, naming,
  and tests. Use during refactors, readability reviews, or when the user asks for
  cleaner or more maintainable code without a specific architecture topic.
---

# Rust — clean code

## Practical principles (community-aligned)

- **Readability over micro-optimization** unless a path is measured as hot.
- **Explicit over implicit**: clear return types; visible conversions (`.into()`, `TryFrom`).
- **Errors as values**: propagate `Result`; prefer `?` over nested `if let Err`.
- **Less shared mutable state**; when it exists, keep boundaries clear (`Arc`, channels, or single-owner design).

## Review checklist

- [ ] Does the function fit one screen and does its name describe the observable effect?
- [ ] Is every `pub` justified, or can visibility shrink?
- [ ] Any `unwrap`/`expect` outside tests or documented invariants?
- [ ] Do tests assert observable behavior, not brittle implementation details?

## Warning signs

- Many parameters of the same primitive type → context struct or builder.
- Heavy `clone` to appease the borrow checker without revisiting API boundaries → redesign ownership at the boundary.
