---
name: rust-best-practices
description: >-
  Idiomatic Rust and engineering hygiene: Clippy, Option/Result patterns,
  ownership in APIs, iterators, unsafe discipline, async Send boundaries, and
  public API docs. Use when writing or reviewing Rust, onboarding idioms, or
  when the user asks for Rust best practices, idioms, or lint hygiene.
---

# Rust — best practices

## Idioms and tooling

- Treat **compiler + Clippy** as part of the review: new warnings or denies should be resolved, not accumulated.
- For **where** format and lint flags live (CI, aliases, allow policy), see [rust-linter-configuration/SKILL.md](../rust-linter-configuration/SKILL.md).
- Use **type-driven design**: let the type system encode states (e.g. builder, sealed traits, newtypes) instead of runtime checks alone.
- Prefer **explicit error types** at boundaries; map to domain errors inside the core rather than leaking `io::Error` everywhere.

## Checklist

- [ ] API takes the weakest ownership (`&`, `&mut`, `AsRef`) that still works?
- [ ] No `unwrap`/`expect` on paths that can fail in production?
- [ ] Every `unsafe` block has a `// SAFETY:` note and minimal scope?
- [ ] Async code does not hold `MutexGuard` or other non-`Send` state across `.await`?
- [ ] New `pub` items have a one-line doc when behavior is not obvious from the name?

## Async and concurrency

- Document or bound **spawn** sites with the same care as public functions (`Send`, `'static` where required).
- Prefer structured concurrency (scoped tasks, clear shutdown) over fire-and-forget where lifecycle matters.

## When in doubt

Prefer code that a mid-level Rust reader understands without reading three layers of macros.
