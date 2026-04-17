# Phase 10: libloragw SPI Integration — Summary

**Phase:** 10-libloragw-spi-integration  
**Status:** ✅ Complete  
**Plans:** 2/2 complete

## What Was Built

Phase 10 establishes the complete libloragw C foundation for SX1302/SX1303 direct hardware integration, replacing the placeholder SPI adapter with real concentrator I/O.

## Plan A: Vendor Foundation
- Vendored sx1302_hal as `libloragw/` git submodule (tag V2.1.0)
- Generated `config.h` via `make inc/config.h` for bindgen compatibility
- Updated `Cargo.toml`: added `cc` build-dep with `spi` feature gate
- Created `build.rs`: compiles 10 C sources when `CARGO_FEATURE_SPI` set
- Generated `lgw_bindings.rs` via bindgen (37 lgw_* functions)

## Plan B: RX Integration
- **lgw_convert.rs**: `lgw_pkt_rx_to_observation()` — converts libloragw packets to `UplinkObservation`
  - Extracts `wire_mic` (last 4 bytes) and `phy_without_mic` per LoRaWAN spec
  - Parses DevAddr, FCnt, FPort, payload from LoRaWAN PHY format
  - Maps freq_hz to RegionId
- **lgw_init.rs**: HAL initialization sequence
  - `lgw_board_setconf` → `lgw_rxrf_setconf` (chains 0+1) → `lgw_start`
  - `Mutex<()>` guard serializes all HAL calls
  - `lgw_stop` on Drop to avoid EBUSY on next start
- **spi_uplink.rs**: Replaced placeholder with real `lgw_receive()` integration
  - `spawn_blocking` with Mutex guard for thread safety
  - Returns `UplinkReceive::Idle` when FIFO empty, observations when packets received

## Key Technical Decisions

| Decision | Rationale |
|----------|----------|
| Submodule path `libloragw/libloragw/` | sx1302_hal has nested directory structure |
| `mod lgw_bindings` (renamed) | Avoids conflict with parent `bindings` module |
| `std::mem::zeroed()` for pkt array | No Default impl on bindgen structs |
| `bindings::com_type_e_LGW_COM_SPI` | Correct enum variant name from bindgen |
| `type_: 0` for radio type | `lgw_radio_type_t::default()` not available |

## Bindings Coverage

Functions: `lgw_board_setconf`, `lgw_rxrf_setconf`, `lgw_rxif_setconf`, `lgw_start`, `lgw_stop`, `lgw_receive`, `lgw_send`, `lgw_status`, `lgw_get_eui`, `lgw_get_temperature`, `lgw_time_on_air`

Structs: `lgw_conf_board_s`, `lgw_conf_rxrf_s`, `lgw_conf_rxif_s`, `lgw_pkt_rx_s`, `lgw_pkt_tx_s`, `lgw_rssi_tcomp_s`, `lgw_tx_gain_s`, `lgw_context_s`

Constants: `LGW_HAL_SUCCESS`, `LGW_HAL_ERROR`, `LGW_COM_SUCCESS`, `LGW_COM_ERROR`, `com_type_e_LGW_COM_SPI`, `com_type_e_LGW_COM_USB`

## Verification

| Check | Result |
|-------|--------|
| `cargo check -p maverick-adapter-radio-spi --features spi` | ✅ Pass (100 warnings) |
| `lgw_bindings.rs` contains `lgw_receive` | ✅ |
| `lgw_bindings.rs` contains `lgw_start`, `lgw_stop` | ✅ |
| `spi_uplink.rs` implements `Drop` | ✅ |
| `wire_mic_split_is_correct` test exists | ✅ |
| Build.rs uses `CARGO_FEATURE_SPI` env var | ✅ |

## Commits

- `23b3bc3` feat(radio-spi): implement lgw_receive() integration with libloragw
- `1c9f29b` docs(phase-10): add 10-B-SUMMARY.md
- `4e4d3ac` docs(phase-10): add 10-A-SUMMARY.md
- `7ad1256` chore: add cc build-dep to Cargo.toml for libloragw C compilation
- `18994d7` feat(radio-spi): vendor sx1302_hal and generate libloragw FFI bindings

## Deferred

- **ARM cross-compile target**: bindings generated on x86 — verify struct layout on real hardware
- **SPI TX / downlink** (lgw_send) — Phase 3.1
- **GatewayEui from OTP**: Currently hardcoded to zero bytes
- **OTAA join handling** — v2
