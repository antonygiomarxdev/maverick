# LNS declarative configuration (`lns-config.toml`)

**Source of truth (operator-edited):** `/etc/maverick/lns-config.toml`  
**Runtime mirror:** SQLite (`<MAVERICK_DATA_DIR>/maverick.db`) via `maverick-edge config load`

## Schema (`schema_version = 1`)

This is the **initial** LNS file contract for Maverick edge: explicit **OTAA vs ABP** per device, optional `dev_addr` for OTAA, and required `dev_addr` for ABP. If you have an obsolete hand-written file from pre-release experiments, replace it with `maverick-edge config init --force` and re-enter devices.

- **`autoprovision`**
  - `enabled` — unknown `DevAddr` creates a **pending** row (not an active device).
  - `rate_limit_per_gateway_per_minute` — cap new pending registrations per gateway per minute (`0` = unlimited in-process bucket).
  - `pending_ttl_secs` — advisory TTL for tooling; pruning is operator-driven.
- **`applications`** — `id`, `name`, `default_region` (must be a known `RegionId`, e.g. `EU868`).
- **`devices`** — each row has:
  - **`activation_mode`**: `otaa` or `abp` (lowercase in TOML).
  - **`dev_eui`** — 16 hex chars (64-bit EUI).
  - **`dev_addr`** — 8 hex chars for **ABP** (required). For **OTAA**, omit or leave empty until you assign a static address or materialize a session after join/approve flows.
  - **`application_id`** — must exist in `applications`.
  - **`region`**, **`enabled`**.
  - **`[devices.otaa]`** — required for `activation_mode = "otaa"`: `join_eui`, `app_key`, optional `nwk_key` (hex strings).
  - **`[devices.abp]`** — optional for `activation_mode = "abp"`: `apps_key`, `nwks_key` (hex strings; optional until downlink/crypto paths consume them).

- **Optional `[radio]`** (Phase 2+): selects the uplink ingest backend. If the section is **absent**, behavior matches older configs (GWMP/UDP). When present:
  - `backend` — `udp` or `spi`.
  - `spi_path` — required when `backend = spi` (e.g. `/dev/spidev0.0`).

Hardware compatibility notes (SPI boards, concentrators) live in [`hardware-registry.toml`](hardware-registry.toml) — a human-editable list, not parsed by the runtime in v1.

### Materialized sessions (`config load`)

- **ABP** devices with `enabled = true` always get a `sessions` row keyed by `dev_addr`.
- **OTAA** devices only get a `sessions` row when **`dev_addr`** is set in the file (e.g. after you know the assigned address). OTAA rows without `dev_addr` are stored in `lns_devices` for keys/metadata but do not create a session until `dev_addr` is present.

## CLI

| Command | Purpose |
|--------|---------|
| `maverick-edge config init` | Write starter file (`--force` to overwrite). |
| `maverick-edge config validate` | Parse + semantic validation only. |
| `maverick-edge config load` | Transactional upsert into SQLite + materialize `sessions` for enabled devices that have a `dev_addr`. |
| `maverick-edge config show` | JSON: policy + mirrored rows. |
| `maverick-edge config list-apps` / `list-devices` / `list-pending` | JSON lists (`list-devices` includes `activation_mode`; `dev_addr_hex` may be null for OTAA without address). |
| `maverick-edge config approve-device` | Promote pending → `lns_devices` + session. |
| `maverick-edge config reject-device` | Drop a pending `DevAddr`. |

Global flag: `--data-dir` / `MAVERICK_DATA_DIR` for the SQLite path.

## Maverick console menu (optional)

When the **Maverick console** is installed (`maverick` / `maverick-edge-tui`), open the interactive menu and choose **`[l] LoRaWAN / LNS`**. From there you can:

- Run the same **`config show` / `validate` / `list-*` / `load`** shortcuts as the CLI.
- Use **guided wizards** to edit **applications**, **devices**, and **autoprovision** while keeping `/etc/maverick/lns-config.toml` as the canonical file (validate via `LnsConfigDocument::validate` before save).
- Device wizard: choose **OTAA vs ABP**, pick an **existing application** from a numbered list (or manual id), and use **[s] Save + optional load** so SQLite matches the file.

**Applications wizard (integrity):**

- **Rename `applications[].id`:** if any `devices[].application_id` still points at the old id, the console shows a **preview** (count and sample `dev_eui` rows) and asks for confirmation; on confirm, those device rows are updated to the new id so validation stays consistent (`application_id` must always match an existing application `id`).
- **Remove an application:** not allowed while **any device** still references that application id. Reassign or remove those devices in the **Devices** wizard first, then remove the application.

If the console cannot write under `/etc/maverick/` (permissions), it may write a copy under `/tmp` and print a `sudo cp` hint.

## Ingest behavior

- Uplink with a **known session** (from SQLite after `config load`, keyed by `DevAddr`) is validated (region vs session, FCnt) and stored with optional `application_id` on uplinks.
- Unknown `DevAddr` with autoprovision **on**: rate-limit check → insert/update `lns_pending` → audit `pending_registration` → actionable error (no uplink stored until approved or configured).
- Unknown with autoprovision **off**: domain error (no session).

## RAK Pi / SSH

```bash
sudo maverick-edge config init --config-path /etc/maverick/lns-config.toml
# edit applications/devices
sudo MAVERICK_DATA_DIR=/var/lib/maverick maverick-edge config load --config-path /etc/maverick/lns-config.toml
sudo systemctl status maverick-edge
```

Use a stable data directory in production (e.g. `MAVERICK_DATA_DIR=/var/lib/maverick`) so `status` / SQLite paths match `config load`.

### Ingest logs on systemd

```bash
sudo journalctl -u maverick-edge.service -n 200 --no-pager
sudo journalctl -u maverick-edge.service -f
```

Optional: `Environment=RUST_LOG=info` (or `debug`) in a `systemctl edit` drop-in for more `tracing` detail.

## See also

- [`compatibility-matrix.md`](compatibility-matrix.md) — tested vs theoretical gateway/radio stacks (evidence template).
- [`runbook-edge.md`](runbook-edge.md) — `probe` / `status` / `health` and ingest troubleshooting.
