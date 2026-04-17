---
phase: 09
plan: A
title: SPI Hardware Detection
type: execute
wave: 1
depends_on: []
autonomous: true
files_modified:
  - crates/maverick-runtime-edge/src/runtime_capabilities.rs
  - crates/maverick-core/src/lns_config.rs
requirements_addressed:
  - CORE-03
  - CORE-04
  - RADIO-01
  - RADIO-02
  - RADIO-03
  - RADIO-04
---

<objective>
Extend RadioEnvironmentHints to detect SX1302/SX1303 SPI concentrator hardware on Linux hosts by probing /dev/spidev* device nodes. Results are surfaced in RuntimeCapabilityReport and probe JSON output.
</objective>

<tasks>

<task type="execute">
<read_first>
- crates/maverick-runtime-edge/src/runtime_capabilities.rs
- crates/maverick-core/src/lns_config.rs
- docs/hardware-registry.toml
</read_first>
<action>
In `crates/maverick-runtime-edge/src/runtime_capabilities.rs`:

1. Add new struct `SpiHardwareHints` to `RadioEnvironmentHints`:
```rust
/// SPI concentrator hardware detected on this host.
#[derive(Debug, Clone, Serialize)]
pub struct SpiHardwareHints {
    /// Paths to accessible SPI devices that match spidev pattern.
    pub available_devices: Vec<String>,
    /// SPI devices that appear to be LoRa concentrators (matched against hardware-registry.toml patterns).
    pub concentrator_candidates: Vec<ConcentratorCandidate>,
    /// Human-readable notes for operator.
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConcentratorCandidate {
    pub spi_path: String,
    pub matched_board: Option<String>,
    pub concentrator_model: Option<String>,
}
```

2. Add `spi_hardware: Option<SpiHardwareHints>` field to `RadioEnvironmentHints`:
```rust
pub struct RadioEnvironmentHints {
    pub platform: &'static str,
    pub systemd_runtime_present: bool,
    pub packet_forwarder_service_hints: Vec<String>,
    /// SPI concentrator hardware detected (None if no SPI hardware found).
    pub spi_hardware: Option<SpiHardwareHints>,
    pub notes: Vec<String>,
}
```

3. Implement `probe_spi_hardware() -> Option<SpiHardwareHints>` function:
```rust
fn probe_spi_hardware() -> Option<SpiHardwareHints> {
    let mut available_devices = Vec::new();
    let mut concentrator_candidates = Vec::new();
    let mut notes = Vec::new();

    // Probe /dev/spidev* device nodes
    let spidev_entries = std::fs::read_dir("/dev").ok()?
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name();
            let name_str = name.to_string_lossy();
            name_str.starts_with("spidev")
        })
        .collect::<Vec<_>>();

    if spidev_entries.is_empty() {
        notes.push("No SPI devices found (/dev/spidev*). SPI ingest not available.".to_string());
        return None;
    }

    for entry in spidev_entries {
        let path = entry.path();
        let path_str = path.display().to_string();
        available_devices.push(path_str.clone());

        // Check if device is accessible (readable - concentrators need read access)
        if std::fs::metadata(&path).map(|m| m.permissions().readonly()).unwrap_or(true) {
            notes.push(format!("{} exists but is not accessible (permission denied)", path_str));
            continue;
        }

        // Check if this looks like a concentrator by matching against hardware-registry.toml patterns
        // For now, match /dev/spidev0.0 and /dev/spidev0.1 as likely concentrator paths
        if path_str == "/dev/spidev0.0" || path_str == "/dev/spidev0.1" {
            concentrator_candidates.push(ConcentratorCandidate {
                spi_path: path_str,
                matched_board: Some("RAK LoRa HAT (detected by path)".to_string()),
                concentrator_model: Some("sx1302 (inferred)".to_string()),
            });
        }
    }

    if concentrator_candidates.is_empty() && !available_devices.is_empty() {
        notes.push("SPI devices found but none match known concentrator patterns.".to_string());
    }

    Some(SpiHardwareHints {
        available_devices,
        concentrator_candidates,
        notes,
    })
}
```

4. Update `RadioEnvironmentHints::probe()` to call `probe_spi_hardware()`:
```rust
fn probe() -> Self {
    let platform = current_platform_label();
    let systemd_runtime_present = Path::new("/run/systemd/system").exists();
    let mut notes = Vec::new();
    let packet_forwarder_service_hints = if cfg!(target_os = "linux") {
        probe_linux_forwarder_hints(&mut notes)
    } else {
        notes.push(
            "Packet-forwarder service scan skipped (non-Linux host); GWMP UDP remains available."
                .to_string(),
        );
        Vec::new()
    };

    // NEW: Probe for SPI hardware
    let spi_hardware = if cfg!(target_os = "linux") {
        probe_spi_hardware()
    } else {
        None
    };

    if packet_forwarder_service_hints.is_empty() && cfg!(target_os = "linux") {
        notes.push(
            "No common packet-forwarder units matched heuristics; confirm your forwarder targets the GWMP bind."
                .to_string(),
        );
    }
    Self {
        platform,
        systemd_runtime_present,
        packet_forwarder_service_hints,
        spi_hardware,
        notes,
    }
}
```

5. Update `RuntimeCapabilityReport::format_operator_summary()` to display SPI hardware:
```rust
// Add after radio_environment section:
if let Some(ref spi) = self.radio_environment.spi_hardware {
    let _ = writeln!(s, "  SPI hardware:");
    let _ = writeln!(s, "    available_devices: {}", spi.available_devices.len());
    for dev in &spi.available_devices {
        let _ = writeln!(s, "      - {}", dev);
    }
    if !spi.concentrator_candidates.is_empty() {
        let _ = writeln!(s, "    concentrator_candidates: {}", spi.concentrator_candidates.len());
        for cand in &spi.concentrator_candidates {
            let board = cand.matched_board.as_deref().unwrap_or("unknown");
            let model = cand.concentrator_model.as_deref().unwrap_or("unknown");
            let _ = writeln!(s, "      - {} ({}, {})", cand.spi_path, board, model);
        }
    }
    for note in &spi.notes {
        let _ = writeln!(s, "    - {}", note);
    }
} else {
    let _ = writeln!(s, "  SPI hardware: none detected");
}
```
</action>
<acceptance_criteria>
- RadioEnvironmentHints::probe() includes spi_hardware field (serde serialized)
- probe_spi_hardware() returns Some(SpiHardwareHints) when /dev/spidev* devices exist
- probe_spi_hardware() returns None when no SPI devices found
- RuntimeCapabilityReport::format_operator_summary() displays SPI hardware info
- Code compiles with cargo build
- No new test failures
</acceptance_criteria>
<verify>
cargo build --package maverick-runtime-edge 2>&1 | grep -E "error|warning:" || echo "BUILD OK"
cargo test --package maverick-runtime-edge 2>&1 | tail -20
</verify>
</task>

</tasks>

<verification>
1. Run `maverick-edge probe --summary` and verify SPI hardware section appears
2. Run `maverick-edge probe` (JSON) and verify spi_hardware field in output
3. Verify probe output on x86 dev machine shows "SPI hardware: none detected"
</verification>

<success_criteria>
- maverick-edge probe --summary shows SPI hardware detection results
- maverick-edge probe JSON output includes spi_hardware field
- SPI hardware detection is best-effort (never fails startup)
- Detection runs on every startup / config reload
</success_criteria>
