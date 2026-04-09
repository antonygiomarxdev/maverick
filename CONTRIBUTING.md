# Contributing to Maverick

Thanks for your interest in contributing.

## Before You Start

- Read the architecture docs in `docs/`.
- Open an issue before large changes.
- Keep changes focused and incremental.

## Local Setup

1. Install stable Rust toolchain.
2. From repository root, run:

```bash
cargo check
cargo test --all
```

## Development Rules

- Preserve hexagonal boundaries: use cases depend on ports, not adapters.
- Keep naming in English and consistent.
- Add tests for all behavior changes.
- Avoid unrelated refactors in feature PRs.

## Pull Request Checklist

- [ ] Code compiles with `cargo check`.
- [ ] Tests pass with `cargo test --all`.
- [ ] Formatting and linting pass.
- [ ] Docs updated when behavior changes.
- [ ] PR description includes motivation and verification steps.

## Commit Guidelines

Use clear, imperative commit messages.
Example: `Add downlink retry transition in sqlite repository`.

## Reporting Security Issues

Do not open public issues for vulnerabilities.
Please follow `SECURITY.md`.
