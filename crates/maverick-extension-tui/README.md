# maverick-extension-tui (Maverick console)

Optional terminal UX extension for Maverick operators.

## What it is

- **Public command:** `maverick` (symlink to `maverick-edge-tui`, created by `scripts/install-linux.sh` when the tarball includes the console binary).
- **Technical binary:** `maverick-edge-tui` (stable artifact name in releases during `0.x` beta).
- Keeps **`maverick-edge`** as the default operator interface; the console adds menus, doctor flow, and guided `setup`.

Invoking the binary as `maverick-edge-tui` prints a short **deprecation notice**; prefer `maverick`.

## Architecture note

This crate is a **secondary composition root for operators**: it orchestrates subprocess calls to `maverick-edge` and local UX. It deliberately **does not** define ingestion or persistence ports; those live in `maverick-core` and `maverick-runtime-edge`. Reuse **types and validation** from `maverick-core` (for example LNS TOML) instead of duplicating domain rules here.

## Scope

- Interactive `setup` wizard (`maverick setup`) and non-interactive defaults
- Status / health / doctor dashboards
- **`[l] LoRaWAN / LNS`**: CLI shortcuts (`config show`, `validate`, `load`, lists) plus guided editors for **applications**, **devices** (**OTAA** vs **ABP**, app picker + manual id), and **autoprovision** (writes `/etc/maverick/lns-config.toml`, optional `config load`)
- Ingest-loop launch using saved configuration
- Loads **system onboarding** from `/etc/maverick/runtime.env` and `/etc/maverick/setup.json` when present, then overlays `~/.config/maverick/tui-config.json`

## Install

Shipped in Maverick Linux release tarballs next to `maverick-edge`. Use the [install script](../../scripts/install-linux.sh) for onboarding + symlink creation.

See also:

- [`docs/install.md`](../../docs/install.md)
- [`docs/extensions.md`](../../docs/extensions.md)
- [`docs/maverick-console-ux-spec.md`](../../docs/maverick-console-ux-spec.md)

## Versioning

Version-locked with `maverick-edge` on the **same Git tag** for `v1.x` releases.

## Run

```bash
maverick
maverick --help
```

Bridge from core CLI:

```bash
maverick-edge setup   # delegates to maverick console (prefers `maverick`, falls back to `maverick-edge-tui`)
```
