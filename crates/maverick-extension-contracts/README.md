# maverick-extension-contracts

Stable contract types for Maverick extension and sync boundaries.

## Purpose

This crate contains versioned envelope and contract types used across core and extension boundaries, with forward-compatible behavior for v1.x.

## Design intent

- Keep contracts explicit and typed
- Avoid runtime coupling between core and optional adapters
- Preserve compatibility guarantees across patch/minor releases

## Versioning

This crate follows workspace tags and is released together with the rest of the monorepo.

Compatibility and version-lock policy:

- see `docs/extensions.md`
