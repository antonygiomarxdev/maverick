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
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## Development Rules

- **`maverick-extension-tui` (Maverick console)** is an **operator-facing composition root**: it shells out to `maverick-edge`, reads onboarding files, and drives menus. It is **not** part of the ingestion hexagon (`maverick-core` ports + adapters). Keep domain rules and persistence behind `maverick-core` / `maverick-edge`; keep the console thin and modular (see [`crates/maverick-extension-tui/README.md`](crates/maverick-extension-tui/README.md)).
- Preserve hexagonal boundaries: use cases depend on ports, not adapters.
- Keep naming in English and consistent.
- Add tests for all behavior changes.
- Avoid unrelated refactors in feature PRs.

## Hardware / gateway compatibility reports

- Canonical policy: [`docs/compatibility-matrix.md`](docs/compatibility-matrix.md) (**tested** vs **theoretical** vs **not supported**).
- To promote a board or radio path to **tested**, include reproducible evidence (template in that doc) and, when possible, output from `scripts/e2e-rakpi-prepush.sh` on the target device.

## Pull Request Checklist

- [ ] Code compiles with `cargo check`.
- [ ] Tests pass with `cargo test --workspace`.
- [ ] Formatting passes with `cargo fmt --all -- --check`.
- [ ] Linting passes with `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- [ ] Docs updated when behavior changes.
- [ ] PR description includes motivation and verification steps.

## Commit Guidelines

Use clear, imperative commit messages.
Example: `Add downlink retry transition in sqlite repository`.

## Reporting Security Issues

Do not open public issues for vulnerabilities.
Please follow `SECURITY.md`.

## Issue Hygiene

- Use status labels on tracked work:
  - `status:planned`
  - `status:in-progress`
  - `status:blocked`
  - `status:done`
- Assign a version milestone for committed scope (`v0.1.x`), or `backlog` when not committed.
- Close duplicates with a link to the canonical issue.
- For stale items, leave a warning comment first and close only after an explicit inactivity window.
- Keep closure comments evidence-based (tests, docs, PR links) to preserve traceability.

### Monthly triage ritual

- Review open issues and milestone alignment.
- Close completed issues with evidence links.
- Re-label blocked/planned items based on current priorities.
