# Phase 10 Plan B: libloragw RX Integration — Summary

## What Was Built

Phase 10 Plan B replaces the placeholder `blocking_poll` with real `lgw_receive()` integration, completing the SPI concentrator RX path.

## Implementation

### B-1: Create lgw_convert.rs
`lgw_pkt_rx_to_observation()` converts libloragw packets to Maverick domain types:
- **wire_mic**: Last 4 bytes of `lgw_pkt_rx_s.payload` — MIC bytes for verification
- **phy_without_mic**: All bytes except last 4 — LoRaWAN PHY without MIC
- **DevAddr**: Bytes [1..5] of payload (little-endian u32)
- **FCnt**: Bytes [6..7] (u16 wire value)
- **FPort**: Byte after FHDR+FOpts (based on FCtrl bit 5)
- **Region**: Derived from `freq_hz` via frequency range matching (AU915/AS923/EU868/EU433/US915)

### B-2: Create lgw_init.rs
HAL initialization sequence for SX1302/SX1303:
- `lgw_board_setconf()` — board config (SPI type, clock source 1 for SX1250)
- `lgw_rxrf_setconf(0)` — RF chain 0 at 867.5 MHz
- `lgw_rxrf_setconf(1)` — RF chain 1 at 868.5 MHz  
- `lgw_start()` — opens SPI and begins RX
- `lgw_stop()` — called on Drop to avoid EBUSY on next start
- `Mutex<()>` guard serializes all HAL calls

### B-3: Refactor spi_uplink.rs
Replaced placeholder with real implementation:
- `SpiUplinkSource::new()` calls `lgw_hal_start()` on construction
- `blocking_receive()` holds `hal_lock` Mutex, calls `lgw_receive(max_pkt=16, pkt_data)`
- `count < 0` → error, `count == 0` → `UplinkReceive::Idle`, `count > 0` → convert and return observations
- `Drop` implementation calls `lgw_hal_stop()` for clean exit
- `async fn next_batch()` uses `spawn_blocking` with `self.clone()`

## Key Bindings Types
- `lgw_pkt_rx_s` — packet receive struct with freq_hz, payload[256], rssic, snr, etc.
- `lgw_conf_board_s` — board config with com_type, com_path[64], lorawan_public, clksrc
- `lgw_conf_rxrf_s` — RF chain config with enable, freq_hz, rssi_offset, tx_enable

## Bindings Generation
- Generated on x86 with GCC stdbool.h include path
- Module renamed to `lgw_bindings` to avoid conflict with parent `bindings` module
- ARM cross-compile target not installed — struct layout verification needed on real hardware

## Verification
| Check | Result |
|-------|--------|
| `cargo check -p maverick-adapter-radio-spi --features spi` | Pass (100 warnings) |
| `bindings.rs` contains `lgw_receive` | ✓ |
| `bindings.rs` contains `lgw_start`, `lgw_stop` | ✓ |
| `bindings.rs` contains `lgw_board_setconf`, `lgw_rxrf_setconf` | ✓ |
| `spi_uplink.rs` implements `Drop` | ✓ |
| `wire_mic_split_is_correct` test exists | ✓ |

## Notes
- `GatewayEui` hardcoded to zero — real implementation reads from concentrator OTP or config
- `lgw_receive(max_pkt=16, pkt_data)` fetches up to 16 packets per poll
- `idle_timeout` (100ms default) controls sleep between empty polls
- Phase 3.1 will add `lgw_send()` TX path using same HAL
