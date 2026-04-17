---
phase: 02-radio-abstraction-spi
plan: D
type: execute
wave: 2
depends_on:
  - 02-A-PLAN.md
files_modified:
  - docs/hardware-registry.toml
  - docs/lns-config.md
  - scripts/install-linux.sh
autonomous: true
requirements:
  - CORE-04
  - RADIO-04

must_haves:
  truths:
    - "A TOML hardware registry ships in-repo (e.g. under `docs/`) with verified / untested / unsupported classification per entry"
    - "At least one entry documents RAK Pi HAT / RAK concentrator as verified-supported with arch and SPI device hints"
    - "Install or release docs mention where the registry lives so operators can extend it without recompiling"
  artifacts:
    - path: "docs/hardware-registry.toml"
      provides: "Community-extensible hardware list"
      contains: "board_name"
---

<objective>
Ship the hardware compatibility registry as human-editable TOML (not compiled into the binary) and wire documentation so CORE-04 / RADIO-04 are satisfied as operator-facing artifacts.

Purpose: Matches CONTEXT D-14–D-17 — runtime does not need to parse this file in v1.
</objective>

<execution_context>
@.planning/REQUIREMENTS.md
@docs/install.md
</execution_context>

<tasks>

<task type="auto">
  <name>Task D-1: Author `hardware-registry.toml`</name>
  <description>
    - Schema: `board_name`, `arch` (armv7/aarch64), `spi_device`, `concentrator_model`, `status` (verified/untested/unsupported), `notes`.
    - Include RAK2287/RAK5146 or equivalent RAK Pi HAT row as verified per CONTEXT.
  </description>
</task>

<task type="auto">
  <name>Task D-2: Documentation cross-links</name>
  <description>
    - Add short section to `docs/lns-config.md` or `docs/install.md` pointing to the registry file and `[radio]` section (after Plans A–C land).
    - If `install-linux.sh` bundles docs, ensure the registry path is included in release archive expectations (comment or echo).
  </description>
</task>

</tasks>
