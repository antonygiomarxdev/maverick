---
phase: 09
plan: C
title: TUI Auto-Detection Integration
type: execute
wave: 2
depends_on:
  - 09-A
  - 09-B
autonomous: true
files_modified:
  - crates/maverick-runtime-edge/src/runtime_capabilities.rs
  - crates/maverick-runtime-edge/src/cli.rs
requirements_addressed:
  - CORE-03
---

<objective>
Surface auto-detected SPI hardware in TUI/setup wizard and probe command output. When SPI hardware is detected but no [radio] config exists, offer operator the choice to enable SPI.
</objective>

<tasks>

<task type="execute">
<read_first>
- crates/maverick-runtime-edge/src/runtime_capabilities.rs
- crates/maverick-runtime-edge/src/cli.rs
</read_first>
<action>
In `crates/maverick-runtime-edge/src/runtime_capabilities.rs`:

1. Update `RuntimeCapabilityReport::format_operator_summary()` to add actionable guidance when SPI auto-detect is possible:

Find the section in `format_operator_summary()` after the SPI hardware display and add:

```rust
// After displaying SPI hardware info, add guidance:
if let Some(ref spi) = self.radio_environment.spi_hardware {
    if !spi.concentrator_candidates.is_empty() {
        let _ = writeln!(s);
        let _ = writeln!(s, "  SPI auto-enable available:");
        let _ = writeln!(s, "    To enable SPI ingest, add to lns-config.toml:");
        let _ = writeln!(s, "      [radio]");
        let _ = writeln!(s, "      backend = \"auto\"");
        let _ = writeln!(s, "    Or use explicit path:");
        let _ = writeln!(s, "      [radio]");
        let _ = writeln!(s, "      backend = \"spi\"");
        let _ = writeln!(s, "      spi_path = \"{}\"", spi.concentrator_candidates[0].spi_path);
    }
}
```

2. Add a method to check if SPI is recommended:
```rust
impl RuntimeCapabilityReport {
    /// Returns true if SPI hardware is detected and ingest is currently UDP.
    pub fn spi_recommended_but_not_enabled(&self) -> bool {
        if let Some(ref spi) = self.radio_environment.spi_hardware {
            if !spi.concentrator_candidates.is_empty() {
                return self.selected_ingest.kind == UplinkBackendKind::GwmpUdp;
            }
        }
        false
    }
}
```
</action>
<acceptance_criteria>
- RuntimeCapabilityReport::format_operator_summary() shows SPI auto-enable guidance
- RuntimeCapabilityReport::spi_recommended_but_not_enabled() correctly detects mismatch
- Code compiles with cargo build
</acceptance_criteria>
<verify>
cargo build --package maverick-runtime-edge 2>&1 | grep -E "error|warning:" || echo "BUILD OK"
cargo test --package maverick-runtime-edge 2>&1 | tail -20
</verify>
</task>

<task type="execute">
<read_first>
- crates/maverick-runtime-edge/src/cli.rs
- crates/maverick-runtime-edge/src/commands/mod.rs
</read_first>
<action>
In `crates/maverick-runtime-edge/src/cli.rs` or the probe command implementation:

Update the probe command to surface SPI detection results prominently. The probe command should:

1. When SPI hardware detected AND current ingest is UDP:
   - Print a prominent notice about SPI hardware being available
   - Suggest enabling SPI with `radio.backend = "auto"` or explicit path

2. When SPI hardware detected AND current ingest is SPI:
   - Confirm SPI is active and show the detected path

Example output for detected SPI + UDP:
```
Maverick runtime capabilities (human summary)

  SPI hardware:
    - /dev/spidev0.0 (RAK LoRa HAT, sx1302)

  ⚠ SPI hardware detected but not in use (GWMP UDP active)
  Enable SPI: add to lns-config.toml:
    [radio]
    backend = "auto"

  Snapshot id: 1234567890
```

The implementation should modify the `format_operator_summary()` to include this prominent notice, or add a separate section in the probe command output.
</action>
<acceptance_criteria>
- maverick-edge probe --summary shows prominent SPI detection notice when applicable
- Notice appears only when SPI hardware detected and UDP is active
- Notice includes actionable config snippet
</acceptance_criteria>
<verify>
cargo build --package maverick-runtime-edge 2>&1 | grep -E "error|warning:" || echo "BUILD OK"
</verify>
</task>

</tasks>

<verification>
1. Build and run `maverick-edge probe --summary` on hardware with SPI
2. Verify SPI detection notice appears when applicable
3. Verify notice is absent when SPI already enabled
</verification>

<success_criteria>
- Operator can see SPI hardware detection results in probe output
- When SPI hardware detected, operator gets clear guidance on enabling SPI
- No confusing messages when SPI is already enabled
</success_criteria>
