---
name: rust-no-magic-values
description: >-
  Applies common production Rust practices to remove magic literals: named
  consts, enums, newtypes, and centralized schema strings. Use when writing or
  reviewing Rust, when the user mentions magic strings/numbers, duplicated
  literals, or when designing core versus adapter layers.
---

# Rust — no magic literals

## What serious codebases typically do

- **Compiler and Clippy**: favor `const` and rich types; teams often enable lints that reduce fragile `unwrap` patterns.
- **`const` / `static`**: protocol values and defaults live in dedicated modules so the meaning of each number has one home.
- **Enums**: closed sets with exhaustive `match`; prevent typos and drift across call sites.
- **Newtypes**: `struct X(u32)` or typed arrays (`Eui64`) to avoid mixing units or semantics.
- **Adapter layer**: table/column names and SQL fragments as `const` or a `schema` module, not copy-paste.
- **Configuration**: operational policy belongs in config structs or named presets (e.g. install profile), not ad hoc literals inside logic.

## Checklist when changing code

- [ ] Does this number or string encode domain or protocol meaning? → `const` or type.
- [ ] Is it a closed set of variants? → `enum` (+ serde/display where needed).
- [ ] Does the same literal already exist elsewhere? → unify in the right crate’s module.
- [ ] Is it SQLite/HTTP/UDP detail? → keep it in the adapter, but **centralized** there.

## Anti-pattern

A helper that takes `&str` for “event kind” used from many places without an enum: every call site is a future bug.
