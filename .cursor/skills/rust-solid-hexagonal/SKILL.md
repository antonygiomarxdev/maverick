---
name: rust-solid-hexagonal
description: >-
  Applies SOLID and hexagonal architecture in Rust: traits as ports,
  implementations in adapters, dependency inversion. Use when designing
  maverick-core/domain versus adapters, adding persistence or transport, or when
  the user mentions SOLID, ports, adapters, or coupling.
---

# Rust — SOLID and hexagonal

## SOLID mapped to Rust

| Principle | In Rust |
|-----------|---------|
| SRP | Focused crates/modules/`impl`; cohesive use-case types |
| OCP | New `enum` variants or new `impl Trait for Struct` |
| LSP | Trait impls honor docs; no surprising panics in “total” methods |
| ISP | Several small traits under `ports/`, not one huge persistence trait |
| DIP | Core defines traits; infrastructure implements them |

## Layout in this kind of monorepo

- **Domain / core**: entities, rules, repository and transport **traits**.
- **Adapters**: sqlite, udp, … — only here do drivers and external schema strings appear.
- **Runtime / binary**: wires concrete implementations into the core.

## Checklist when adding a capability

- [ ] Does core import any I/O crate? → hide it behind a trait.
- [ ] Does the trait force adapters to know too much internal detail? → split the trait (ISP).
- [ ] Does an adapter change break domain tests? → domain tests should use fakes or test doubles for traits, not real SQLite.
