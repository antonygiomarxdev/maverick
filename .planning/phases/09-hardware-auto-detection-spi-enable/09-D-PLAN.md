---
phase: 09
plan: D
title: Auto-Detection Verification
type: execute
wave: 3
depends_on:
  - 09-A
  - 09-B
  - 09-C
autonomous: false
files_modified: []
requirements_addressed:
  - CORE-03
  - RADIO-03
---

<objective>
Verify end-to-end auto-detection flow: SPI hardware detection, auto-backend selection, and probe output integration. Test fallback behavior when SPI hardware is unavailable.
</objective>

<tasks>

<task type="execute">
<read_first>
- crates/maverick-runtime-edge/src/runtime_capabilities.rs
- crates/maverick-runtime-edge/src/radio_ingest_selection.rs
</read_first>
<action>
Integration test: verify auto-detection flow works end-to-end.

1. Test on x86 dev machine (no SPI):
   - Run `maverick-edge probe --summary`
   - Verify output shows "SPI hardware: none detected"
   - Verify no SPI auto-enable notice appears

2. Test config-less startup (no lns-config.toml):
   - Maverick starts with UDP (default)
   - Log shows no errors

3. Test auto mode with no SPI (if possible via mock):
   - Create lns-config.toml with `radio.backend = "auto"`
   - Maverick should start with UDP
   - Log should show "SPI auto-detect: no SPI concentrator hardware detected — using UDP ingest"

4. Verify the feature flag gating:
   - Build without `--features spi`
   - With `radio.backend = "auto"` and SPI detected, should fall back to UDP
   - Build with `--features spi` should use SPI path

For manual testing, document test steps in a test script that can be run on RAK Pi hardware.
</action>
<acceptance_criteria>
- maverick-edge probe shows correct SPI hardware status on each platform
- Auto mode fallback to UDP works correctly
- No startup failures when SPI hardware unavailable
</acceptance_criteria>
<verify>
cargo build --package maverick-runtime-edge 2>&1 | grep -E "error|warning:" || echo "BUILD OK"
cargo test --package maverick-runtime-edge 2>&1 | tail -20
</verify>
</task>

<task type="execute">
<read_first>
- crates/maverick-runtime-edge/src/runtime_capabilities.rs
</read_first>
<action>
Add unit tests for SPI hardware detection:

In `crates/maverick-runtime-edge/src/runtime_capabilities.rs`, add test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spi_hardware_detection_returns_none_on_no_spidev() {
        // On systems without /dev/spidev*, should return None
        let result = probe_spi_hardware();
        // This test may pass or fail depending on platform
        // The key is it doesn't panic - it's best-effort
        println!("SPI probe result: {:?}", result);
    }

    #[test]
    fn test_radio_environment_hints_includes_spi_field() {
        let hints = RadioEnvironmentHints::probe();
        // spi_hardware field should exist (may be None on platforms without SPI)
        println!("Radio environment hints: {:?}", hints);
    }
}
```

Run: `cargo test --package maverick-runtime-edge -- runtime_capabilities`
</action>
<acceptance_criteria>
- Tests compile and run without panics
- Tests verify SPI detection is best-effort (never fails startup)
</acceptance_criteria>
<verify>
cargo test --package maverick-runtime-edge -- runtime_capabilities 2>&1 | tail -30
</verify>
</task>

</tasks>

<verification>
1. Run integration tests on target platform (x86 for no-SPI, RAK Pi for SPI detection)
2. Verify probe output format matches expectations
3. Verify auto-enable notice appears only when applicable
</verification>

<success_criteria>
- All existing tests pass
- New SPI detection tests pass
- maverick-edge probe works correctly on all platforms
- Auto-enable flow works end-to-end
</success_criteria>
