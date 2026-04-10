# maverick-extension-tui

Optional terminal UX extension for Maverick operators.

## What it is

`maverick-edge-tui` is an external extension binary that wraps common operator flows while keeping `maverick-edge` CLI as the default interface.

## Scope

- Welcome and basic configuration flow
- Quick status and health checks
- Ingest-loop launch using saved configuration

## Install

The binary is shipped in Maverick Linux release tarballs together with `maverick-edge`.

See:

- `docs/install.md`
- `docs/extensions.md`

## Versioning

This extension follows workspace release tags and is version-locked with the core runtime during v1.x.

Use matching versions of:

- `maverick-edge`
- `maverick-edge-tui`

## Run

```bash
maverick-edge-tui
```
