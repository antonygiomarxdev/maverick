# Agent instructions — Maverick

These Rust engineering standards apply to **any** coding assistant or reviewer (Cursor, VS Code–based tools, Claude Code, Copilot, etc.). Follow them when implementing or reviewing changes in this repository.

## Standards (canonical locations)

| Topic | Details |
|--------|---------|
| No magic literals | [.cursor/rules/rust-no-magic-values.mdc](.cursor/rules/rust-no-magic-values.mdc) · [.cursor/skills/rust-no-magic-values/SKILL.md](.cursor/skills/rust-no-magic-values/SKILL.md) |
| Clean code | [.cursor/rules/rust-clean-code.mdc](.cursor/rules/rust-clean-code.mdc) · [.cursor/skills/rust-clean-code/SKILL.md](.cursor/skills/rust-clean-code/SKILL.md) |
| SOLID & hexagonal | [.cursor/rules/rust-solid-hexagonal.mdc](.cursor/rules/rust-solid-hexagonal.mdc) · [.cursor/skills/rust-solid-hexagonal/SKILL.md](.cursor/skills/rust-solid-hexagonal/SKILL.md) |
| Best practices (idioms, tooling, async) | [.cursor/rules/rust-best-practices.mdc](.cursor/rules/rust-best-practices.mdc) · [.cursor/skills/rust-best-practices/SKILL.md](.cursor/skills/rust-best-practices/SKILL.md) |
| Linter & formatter config | [.cursor/rules/rust-linter-configuration.mdc](.cursor/rules/rust-linter-configuration.mdc) · [.cursor/skills/rust-linter-configuration/SKILL.md](.cursor/skills/rust-linter-configuration/SKILL.md) |
| PR / sprint review gate | [docs/code-review-checklist.md](docs/code-review-checklist.md) — clean code, SOLID/hexagonal, no magic values, verification |

**Repo config:** [`rustfmt.toml`](rustfmt.toml), [root `Cargo.toml` `[workspace.lints]`](Cargo.toml) (shared rustc/Clippy levels; each crate uses `[lints] workspace = true`), [`.cargo/config.toml`](.cargo/config.toml) (`cargo fmt-check`, `cargo lint`), [`.github/workflows/ci.yml`](.github/workflows/ci.yml).

**Cursor:** `.mdc` rules under `.cursor/rules/` apply automatically for matching `globs` when those files are relevant to the session.

**Other tools:** add the rule and/or skill paths above to the session context (e.g. `@` file attach, project docs, or custom instructions). The markdown content is tool-agnostic; only the `.mdc` YAML frontmatter is Cursor-specific metadata.

Language: **English** for all of the above documents.
