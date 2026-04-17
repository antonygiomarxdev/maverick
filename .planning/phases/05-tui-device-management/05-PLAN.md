---
phase: 5
name: TUI Device Management
wave: 1
depends_on: []
autonomous: true
requirements_addressed:
  - DEV-01
  - DEV-02
  - DEV-03
  - DEV-04
  - DEV-05
  - CORE-03
files_modified:
  - crates/maverick-extension-tui/src/lns_wizard.rs
  - crates/maverick-extension-tui/src/menu_lorawan.rs
  - crates/maverick-extension-tui/src/console_ui.rs
  - crates/maverick-extension-tui/src/lns_file.rs
  - crates/maverick-extension-tui/src/probe.rs
  - crates/maverick-runtime-edge/src/commands.rs
---

## Plan: TUI Device Management Implementation

### Objective

Implement full device management through the TUI: add, list, remove devices, promote autoprovision-pending devices, and display hardware probe results. Maintain backward compatibility with lns-config.toml imports.

### Success Criteria

1. **DEV-01**: Operator can add device (DevEUI, DevAddr, keys, region, app) via TUI — immediately active
2. **DEV-02**: Device list shows all devices with last-seen timestamp and uplink count
3. **DEV-03**: Operator can remove device via TUI — subsequent frames rejected
4. **DEV-04**: lns-config.toml import still works — no breaking changes
5. **DEV-05**: Autoprovision-pending devices visible in TUI, can be promoted in one action
6. **CORE-03**: Hardware probe runs on startup, results visible in TUI

### Tasks

#### Task 1: Create Device Management CLI Commands

<read_first>
- crates/maverick-runtime-edge/src/commands.rs
- crates/maverick-domain/src/identifiers.rs
</read_first>

<action>
Add to `commands.rs`:

1. `DeviceAdd` command:
```rust
#[derive(Debug, clap::Args)]
pub struct DeviceAdd {
    #[arg(long)]
    pub dev_eui: String,
    #[arg(long)]
    pub dev_addr: String,
    #[arg(long)]
    pub nwk_s_key: String,
    #[arg(long)]
    pub app_s_key: String,
    #[arg(long)]
    pub region: String,
    #[arg(long)]
    pub application_id: String,
}
```

2. `DeviceList` command:
```rust
#[derive(Debug, clap::Parser)]
pub struct DeviceList;
```

3. `DeviceRemove` command:
```rust
#[derive(Debug, clap::Args)]
pub struct DeviceRemove {
    #[arg(long)]
    pub dev_eui: String,
}
```

4. `DevicePromote` command (for DEV-05):
```rust
#[derive(Debug, clap::Args)]
pub struct DevicePromote {
    #[arg(long)]
    pub dev_addr: String,
}
```

5. `DeviceShow` command:
```rust
#[derive(Debug, clap::Args)]
pub struct DeviceShow {
    #[arg(long)]
    pub dev_eui: String,
}
```

Each command returns JSON to stdout for TUI consumption.
</action>

<acceptance_criteria>
- `maverick-edge device add` creates new device
- `maverick-edge device list` returns all devices with metadata
- `maverick-edge device remove` deletes device
- `maverick-edge device promote` converts pending to registered
- `maverick-edge device show` returns single device details
- All commands output JSON to stdout
</acceptance_criteria>

---

#### Task 2: Implement Device Repository Adapter

<read_first>
- crates/maverick-core/src/ports/device_repository.rs
</read_first>

<action>
Check existing `DeviceRepository` port and implement:

1. Add to `device_repository.rs`:
```rust
#[derive(Debug, Clone)]
pub struct Device {
    pub dev_eui: DevEui,
    pub dev_addr: Option<DevAddr>,
    pub activation_mode: ActivationMode,
    pub application_id: String,
    pub region: RegionId,
    pub enabled: bool,
}
```

2. Extend `DeviceRepository` trait:
```rust
#[async_trait]
pub trait DeviceRepository: Send + Sync {
    async fn add_device(&self, device: &Device) -> AppResult<()>;
    async fn list_devices(&self) -> AppResult<Vec<Device>>;
    async fn get_device(&self, dev_eui: &DevEui) -> AppResult<Option<Device>>;
    async fn remove_device(&self, dev_eui: &DevEui) -> AppResult<()>;
    async fn update_device(&self, device: &Device) -> AppResult<()>;
    async fn list_pending_devices(&self) -> AppResult<Vec<PendingDevice>>;
    async fn promote_pending(&self, dev_addr: DevAddr, keys: SessionKeys) -> AppResult<()>;
}
```

3. Implement SQLite adapter in `maverick-adapter-persistence-sqlite`:
   - Add devices table to schema.sql
   - Add lns_devices operations to repos.rs
</action>

<acceptance_criteria>
- DeviceRepository trait has all CRUD methods plus pending device operations
- SQLite adapter implements DeviceRepository
- Pending device promotion creates session from pending device info
</acceptance_criteria>

---

#### Task 3: Build TUI Device Screens

<read_first>
- crates/maverick-extension-tui/src/menu_lorawan.rs
- crates/maverick-extension-tui/src/console_ui.rs
</read_first>

<action>
Add to TUI:

1. **Device List Screen** (`menu_lorawan.rs`):
   - Display all devices in table format
   - Columns: DevEUI, DevAddr, Region, Last Seen, Uplink Count, Status
   - Arrow keys to navigate, Enter to select
   - 'A' to add new device
   - 'D' to delete selected device
   - 'R' to refresh

2. **Add Device Wizard** (`lns_wizard.rs`):
   - Multi-step form: DevEUI → DevAddr → Keys → Region → Application
   - Validation on each step
   - Summary before confirmation
   - Immediate activation on completion

3. **Device Detail Screen**:
   - Full device info display
   - Uplink history graph
   - Delete button with confirmation

4. **Pending Devices Screen** (for DEV-05):
   - List unknown DevAddr frames seen
   - Show first_seen, gateway_eui
   - "Promote" action with key entry form
</action>

<acceptance_criteria>
- TTY renders device list with navigation
- Add wizard collects all required fields
- Delete requires confirmation
- Pending devices screen shows unknown DevAddrs
</acceptance_criteria>

---

#### Task 4: Hardware Probe Display (CORE-03)

<read_first>
- crates/maverick-extension-tui/src/probe.rs
</read_first>

<action>
1. Extend probe.rs to capture:
   - CPU arch (x86_64, aarch64, armv7)
   - Available RAM (free - total)
   - Storage capacity and available
   - Raspberry Pi model (if applicable)

2. Add probe results to TUI startup screen:
```
┌─────────────────────────────────────────┐
│ Maverick Edge LNS           [Status: ●] │
├─────────────────────────────────────────┤
│ Hardware: Raspberry Pi 4 Model B        │
│ Arch: aarch64    RAM: 3.7 GB free      │
│ Storage: 28.5 GB / 64 GB available      │
│ Region: EU868                          │
├─────────────────────────────────────────┤
│ Devices: 12 active  │  Uplinks: 1,847 │
│ [D]evices  [Q]uit  [H]elp             │
└─────────────────────────────────────────┘
```

3. Auto-run probe on TUI startup (no manual trigger needed).
</action>

<acceptance_criteria>
- Probe runs automatically on TUI startup
- Hardware info displays in header/status bar
- CPU arch, RAM, storage shown
- Probe errors don't crash TUI (graceful degradation)
</acceptance_criteria>

---

#### Task 5: lns-config.toml Import Compatibility (DEV-04)

<read_first>
- crates/maverick-extension-tui/src/lns_file.rs
</read_first>

<action>
1. Read existing lns-config.toml import logic in `lns_file.rs`

2. Verify it covers all required fields:
   - DevEUI, DevAddr, NwkSKey, AppSKey
   - Region, application_id

3. Add import confirmation screen:
```
Import 5 devices from lns-config.toml?
  - Device A: 0001020304050607 (EU868)
  - Device B: 0001020304050608 (US915)
  ...

  [I]mport All  [S]kip  [A]bort
```

4. After import, devices should be immediately active (no restart).
</action>

<acceptance_criteria>
- Existing lns-config.toml files import correctly
- Import shows preview before applying
- Imported devices immediately active
- No breaking changes to existing config format
</acceptance_criteria>

---

### Verification

Run:
```bash
cargo test -p maverick-extension-tui
cargo build -p maverick-extension-tui
cargo test -p maverick-integration-tests
```

Manual verification:
1. Launch TUI: `maverick-edge tui`
2. Check hardware probe displays correctly
3. Add a device via TUI
4. List devices and verify new device appears
5. Remove device and verify it's gone
6. Check pending devices screen
7. Test lns-config.toml import
