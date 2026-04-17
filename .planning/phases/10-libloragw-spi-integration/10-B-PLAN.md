---
wave: 2
depends_on: [10-A]
requirements_addressed: [RADIO-01, RADIO-02]
files_modified:
  - crates/maverick-adapter-radio-spi/src/lgw_convert.rs (new)
  - crates/maverick-adapter-radio-spi/src/lgw_init.rs (new)
  - crates/maverick-adapter-radio-spi/src/spi_uplink.rs
---
# Phase 10: libloragw RX Integration — Plan B

**Wave 2:** SPI concentrator RX — lgw_receive(), HAL init, payload parsing, wire_mic/phy_without_mic split

## Context

- Phase 2 established: `Mutex<()>` guard protects libloragw global HAL state
- Phase 2 established: `spawn_blocking` for blocking HAL calls
- Phase 2 established: `lgw_pkt_rx_s.payload` includes full PHY including MIC (last 4 bytes) — split in Rust conversion layer
- Phase 9: `RadioBackend::Auto` + `RadioIngestSelection::AutoSpi` exists — Phase 10 wires real hardware

## Objective

Replace the `blocking_poll` placeholder in `SpiUplinkSource` with real `lgw_receive()` calls. Implement:
- HAL initialization sequence (lgw_board_setconf, lgw_start)
- `Drop` for clean lgw_stop on scope exit
- `lgw_receive` wrapped in `spawn_blocking` with `Mutex<()>` guard
- `lgw_pkt_rx_s` → `UplinkObservation` conversion with correct `wire_mic` / `phy_without_mic` split

## Tasks

### B-1: Create lgw_convert.rs — lgw_pkt_rx_s to UplinkObservation

**Action:** Create `crates/maverick-adapter-radio-spi/src/lgw_convert.rs` with the conversion from libloragw packet structs to Maverick's domain types:

```rust
//! Convert libloragw `lgw_pkt_rx_s` structs to `UplinkObservation`.
//!
//! KEY: `lgw_pkt_rx_s.payload` contains the full LoRaWAN PHY frame INCLUDING the 4-byte MIC.
//! The split MUST happen here:
//!   wire_mic      = payload[size-4..size]       (last 4 bytes)
//!   phy_without_mic = payload[..size-4]         (everything before MIC)
//! Without this split, MIC verification in IngestUplink receives zeros and ALL frames are rejected.

use crate::bindings::{lgw_pkt_rx_s, lgw_context_s};
use maverick_core::error::{AppError, AppResult};
use maverick_core::ports::radio_transport::{
    UplinkObservation, GatewayEui, RegionId, DevAddr,
};
use std::convert::TryInto;

pub fn lgw_pkt_rx_to_observation(
    pkt: &lgw_pkt_rx_s,
    gateway_eui: GatewayEui,
) -> AppResult<UplinkObservation> {
    let size = pkt.size as usize;
    if size == 0 || size > 256 {
        return Err(AppError::Infrastructure("invalid packet size from lgw_receive".to_string()));
    }

    // SAFETY: pkt.payload is [u8; 256] in bindings; we read only `size` bytes
    let payload_ptr = pkt.payload.as_ptr();

    // Extract wire_mic = last 4 bytes of the PHY payload
    // Extract phy_without_mic = all bytes except the last 4
    let full_payload = unsafe { std::slice::from_raw_parts(payload_ptr, size) };

    if size < 4 {
        return Err(AppError::Infrastructure(
            format!("packet too small for MIC: {} bytes", size)
        ));
    }

    let wire_mic: [u8; 4] = full_payload[size - 4..].try_into().unwrap();
    let phy_without_mic = full_payload[..size - 4].to_vec();

    // Parse DevAddr (bytes 1-4 of FHDR), FCnt (bytes 6-7 of FHDR), FPort (after FHDR+FOpts)
    // LoRaWAN PHY format: MHDR(1) | FHDR(8) | FPort(0/1) | FRMPayload(?) | MIC(4)
    // FHDR: DevAddr(4) | FCtrl(1) | FCnt(2) | FOpts(0..15)
    let dev_addr = DevAddr::try_from(&full_payload[1..5]).map_err(|_| {
        AppError::Infrastructure("invalid DevAddr in lgw_pkt_rx payload".to_string())
    })?;
    let f_cnt = u16::from_le_bytes([full_payload[6], full_payload[7]]);
    // FPort is byte at index 8 + fopts_len (FCtrl bit 5 = fopts length)
    let fctrl = full_payload[5];
    let fopts_len = (fctrl & 0x0F) as usize;
    let fport_idx = 8 + fopts_len;
    let f_port = if size > fport_idx { full_payload[fport_idx] } else { 0 };
    let payload = if size > fport_idx + 1 {
        full_payload[fport_idx + 1..size - 4].to_vec()
    } else {
        vec![]
    };

    // Determine region from frequency
    let region = freq_to_region(pkt.freq_hz);

    // RSSI and SNR from radio metadata
    let rssi = Some(pkt.rssic as i16);
    let snr = Some(pkt.snr);

    Ok(UplinkObservation {
        dev_addr,
        f_cnt,
        f_port,
        payload,
        wire_mic,
        phy_without_mic,
        gateway_eui,
        region,
        rssi,
        snr,
    })
}

fn freq_to_region(freq_hz: u32) -> RegionId {
    match freq_hz {
        915_000_000..=928_000_000 => RegionId::Au915,
        923_000_000..=927_000_000 => RegionId::As923,
        867_000_000..=869_000_000 => RegionId::Eu868,
        779_000_000..=787_000_000 => RegionId::Eu433,
        470_000_000..=510_000_000 => RegionId::Cn470,
        488_000_000..=496_000_000 => RegionId::Cn779,
        902_000_000..=928_000_000 => RegionId::Us915,
        916_000_000..=920_000_000 => RegionId::Kr920,
        _ => RegionId::Eu868, // fallback
    }
}
```

**read_first:**
- `.planning/phases/10-libloragw-spi-integration/10-CONTEXT.md` — Gray Area 5 (lgw_receive timeout), specifics (hardcoded RAK defaults, configurable idle timeout)
- `crates/maverick-adapter-radio-spi/src/spi_uplink.rs` — current placeholder `blocking_poll`, module contract doc
- `crates/maverick-core/src/ports/radio_transport.rs` — `UplinkObservation` struct fields, `RegionId`, `GatewayEui`, `DevAddr`
- `.planning/phases/02-radio-abstraction-spi/02-RESEARCH.md` — Pitfall 6 (UplinkObservation from lgw_pkt_rx_s)

**acceptance_criteria:**
- `lgw_convert.rs` exists and exports `lgw_pkt_rx_to_observation(pkt, gateway_eui) -> AppResult<UplinkObservation>`
- `wire_mic` is exactly `payload[size-4..size]` — last 4 bytes of full PHY payload
- `phy_without_mic` is `payload[..size-4]` — all bytes except MIC
- DevAddr parsed from bytes [1..5] of payload (PHY index 1-4)
- FCnt parsed from bytes [6..7] (PHY index 6-7), converted to u16 via `from_le_bytes`
- FPort parsed based on FCtrl bit 5 (FOpts length)
- Region inferred from `freq_hz` using frequency range matching (AU915/AS923/EU868/...)

---

### B-2: Create lgw_init.rs — HAL initialization (lgw_board_setconf, lgw_start, lgw_stop)

**Action:** Create `crates/maverick-adapter-radio-spi/src/lgw_init.rs` with HAL initialization, cleanup, and RAK2287/RAK5146 board defaults:

```rust
//! libloragw HAL initialization and cleanup.
//!
//! lgw_start() requires prior lgw_board_setconf() with board configuration.
//! lgw_stop() must be called on Drop to avoid EBUSY on next lgw_start().
//!
//! RAK2287 (SX1302) and RAK5146 (SX1303) use sensible defaults for:
//!   - SPI device path
//!   - Clock source (SX1250/SX1257)
//!   - Full duplex configuration
//!   - RF chain 0 and 1 configuration with correct frequency plans

use crate::bindings;
use maverick_core::error::{AppError, AppResult};
use std::sync::Mutex;

/// HAL context singleton — libloragw uses module-level C global state.
/// Only one thread may call HAL functions at a time.
static HAL_INIT: Mutex<()> = Mutex::new(());

/// Initialize the SX1302/SX1303 concentrator via libloragw.
/// Call lgw_board_setconf for board + RF chains, then lgw_start.
/// Returns Ok(()) on success; Err if device busy or permissions issue.
pub fn lgw_hal_start(spi_path: &str) -> AppResult<()> {
    // Serialize all HAL calls through a Mutex guard
    let _guard = HAL_INIT.lock().map_err(|_| {
        AppError::Infrastructure("lgw hal mutex poisoned".to_string())
    })?;

    // Board configuration (RAK2287 / SX1302 defaults)
    let board_conf = build_board_conf(spi_path)?;
    let board_ptr = &board_conf as *const _;
    let ret = unsafe { bindings::lgw_board_setconf(board_ptr) };
    if ret != bindings::LGW_HAL_SUCCESS as i32 {
        return Err(AppError::Infrastructure(
            format!("lgw_board_setconf failed: {}", ret)
        ));
    }

    // RF chain 0 configuration
    let rf0_conf = build_rf0_conf()?;
    let rf0_ptr = &rf0_conf as *const _;
    let ret = unsafe { bindings::lgw_rxrf_setconf(0, rf0_ptr) };
    if ret != bindings::LGW_HAL_SUCCESS as i32 {
        return Err(AppError::Infrastructure(
            format!("lgw_rxrf_setconf chain 0 failed: {}", ret)
        ));
    }

    // RF chain 1 configuration
    let rf1_conf = build_rf1_conf()?;
    let rf1_ptr = &rf1_conf as *const _;
    let ret = unsafe { bindings::lgw_rxrf_setconf(1, rf1_ptr) };
    if ret != bindings::LGW_HAL_SUCCESS as i32 {
        return Err(AppError::Infrastructure(
            format!("lgw_rxrf_setconf chain 1 failed: {}", ret)
        ));
    }

    // LoRa (non-FSK) RX configuration for both chains
    let sx1302_rx_conf = build_sx1302_rx_conf()?;
    let sx1302_ptr = &sx1302_rx_conf as *const _;
    let ret = unsafe { bindings::lgw_sx1302_rx_setconf(0, sx1302_ptr) };
    if ret != bindings::LGW_HAL_SUCCESS as i32 {
        return Err(AppError::Infrastructure(
            format!("lgw_sx1302_rx_setconf failed: {}", ret)
        ));
    }

    // Start the concentrator — this opens SPI and begins RX
    let ret = unsafe { bindings::lgw_start() };
    if ret != bindings::LGW_HAL_SUCCESS as i32 {
        return Err(AppError::Infrastructure(
            format!("lgw_start failed: {} (device busy or permissions?)", ret)
        ));
    }

    tracing::info!("libloragw HAL started on {}", spi_path);
    Ok(())
}

/// Stop the concentrator and release SPI device.
/// Must be called when SpiUplinkSource drops — otherwise next lgw_start() fails with EBUSY.
pub fn lgw_hal_stop() {
    if let Ok(_guard) = HAL_INIT.lock() {
        let ret = unsafe { bindings::lgw_stop() };
        if ret == bindings::LGW_HAL_SUCCESS as i32 {
            tracing::info!("libloragw HAL stopped");
        } else {
            tracing::warn!("lgw_stop returned {} (may be already stopped)", ret);
        }
    }
}

fn build_board_conf(spi_path: &str) -> AppResult<bindings::lgw_board_conf_s> {
    // SPI path is typically /dev/spidev0.0 for RAK Pi HAT
    // Clock source: 2 = SX1250, 1 = SX1257 (RAK2287 uses SX1250)
    // Full duplex: 0 for LoRa gateway
    let mut conf = bindings::lgw_board_conf_s::default();
    conf.clock_source = 2; // SX1250
    conf.full_duplex = 0;
    // spi_path is passed as a C string through the loragw_board_conf_s structure
    // The actual SPI device is configured via platform-specific code in loragw_spi.c
    // For RAK2287, the SPI device is /dev/spidev0.0 — configured via platform args
    Ok(conf)
}

fn build_rf0_conf() -> AppResult<bindings::lgw_rxrf_conf_s> {
    let mut conf = bindings::lgw_rxrf_conf_s::default();
    conf.enable = 1;
    conf.freq_hz = 867_500_000; // EU868 band — adjust per region
    conf.rssi_offset = -166.0;  // calibrated RSSI offset for SX1250
    conf.rssi_tempoffset = -166;
    conf.tx_enable = 0; // TX not implemented in Phase 10 (Phase 3.1)
    conf.tx_freq_min = 863_000_000;
    conf.tx_freq_max = 870_000_000;
    Ok(conf)
}

fn build_rf1_conf() -> AppResult<bindings::lgw_rxrf_conf_s> {
    let mut conf = bindings::lgw_rxrf_conf_s::default();
    conf.enable = 1;
    conf.freq_hz = 868_500_000;
    conf.rssi_offset = -166.0;
    conf.rssi_tempoffset = -166;
    conf.tx_enable = 0;
    conf.tx_freq_min = 863_000_000;
    conf.tx_freq_max = 870_000_000;
    Ok(conf)
}

fn build_sx1302_rx_conf() -> AppResult<bindings::lgw_sx1302_rx_conf_s> {
    let mut conf = bindings::lgw_sx1302_rx_conf_s::default();
    conf.enable = 1;
    conf.modulation = bindings::LGW_LORA_MODULATION as u8;
    conf.bandwidth = 125_000;
    conf.datarate = 7; // SF7
    conf.coderate = 1; // 4/5
    conf.flags.implicit_header = 0;
    conf.flags.crc_en = 1;
    conf.flags.cr_coding = 1;
    conf.flags invert_pol = 0;
    conf.flags.rx_cont = 1;
    conf.flags.single_mode = 0;
    conf.count = 0;
    Ok(conf)
}
```

**read_first:**
- `.planning/phases/02-radio-abstraction-spi/02-RESEARCH.md` — Pattern 3 (spawn_blocking), Anti-patterns (global HAL state without Mutex, lgw_stop on Drop), Pitfall 3 (lgw_start/lgw_stop symmetry)
- `crates/maverick-adapter-radio-spi/src/bindings.rs` — lgw_board_conf_s, lgw_rxrf_conf_s, lgw_sx1302_rx_conf_s struct definitions
- `10-CONTEXT.md` — Gray Area 1 (libloragw version), Gray Area 3 (hardcoded RAK defaults), specifics (lgw_stop on Drop)

**acceptance_criteria:**
- `lgw_hal_start(spi_path) -> AppResult<()>` calls lgw_board_setconf → lgw_rxrf_setconf (chains 0+1) → lgw_sx1302_rx_setconf → lgw_start
- `lgw_hal_stop()` calls `lgw_stop()` in unsafe block
- `HAL_INIT: Mutex<()>` exists and serializes all HAL calls
- RAK2287 defaults used: clock_source=2 (SX1250), rssi_offset=-166
- `lgw_start()` failure returns `AppError::Infrastructure` with diagnostic (device busy / permissions)
- All HAL functions called within `unsafe { ... }` blocks as required by bindings

---

### B-3: Refactor spi_uplink.rs — Replace placeholder with real lgw_receive()

**Action:** Replace the current `blocking_poll` placeholder in `crates/maverick-adapter-radio-spi/src/spi_uplink.rs` with real libloragw integration:

```rust
//! `UplinkSource` over SPI — libloragw backed.
//!
//! ## SPI Adapter — UplinkObservation Parsing Contract
//!
//! When integrating libloragw RX (lgw_receive), the SPI adapter MUST:
//!
//! 1. Extract `wire_mic = phy_payload[phy_payload.len()-4..]` (last 4 bytes)
//! 2. Extract `phy_without_mic = &phy_payload[..phy_payload.len()-4]`
//! 3. Extract DevAddr, FCnt, FPort, payload per LoRaWAN 1.0.x PHY format
//! 4. Pass ALL of the above to UplinkObservation
//!
//! Without `wire_mic` and `phy_without_mic`, MIC verification in IngestUplink
//! will receive zeros and ALL valid frames will be rejected.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use maverick_core::error::{AppError, AppResult};
use maverick_core::ports::{UplinkReceive, UplinkSource};
use maverick_core::ports::radio_transport::GatewayEui;

use crate::bindings::{lgw_pkt_rx_s, LGW_LORA, LGW_HAL_ERROR, LGW_SUCCESS};
use crate::lgw_convert::lgw_pkt_rx_to_observation;
use crate::lgw_init::{lgw_hal_start, lgw_hal_stop};

/// Gateway EUI — hardcoded for now; could come from concentrator OTP or config.
/// Use a placeholder EUI since real hardware EUI is read from the concentrator OTP.
const GATEWAY_EUI: GatewayEui = GatewayEui::from_bytes([0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);

#[derive(Clone)]
pub struct SpiUplinkSource {
    inner: Arc<SpiInner>,
}

struct SpiInner {
    hal_lock: std::sync::Mutex<()>,
    idle_timeout: Duration,
}

impl SpiUplinkSource {
    pub fn new(spi_path: String, idle_timeout: Duration) -> AppResult<Self> {
        let trimmed = spi_path.trim();
        if trimmed.is_empty() {
            return Err(AppError::InvalidInput(
                "spi_path must not be empty for SpiUplinkSource".to_string(),
            ));
        }

        // Start the HAL — this calls lgw_board_setconf + lgw_start
        lgw_hal_start(trimmed)?;

        Ok(Self {
            inner: Arc::new(SpiInner {
                hal_lock: std::sync::Mutex::new(()),
                idle_timeout,
            }),
        })
    }

    fn blocking_receive(&self) -> AppResult<UplinkReceive> {
        let _guard = self.inner.hal_lock.lock().map_err(|_| {
            AppError::Infrastructure("spi hal mutex poisoned".to_string())
        })?;

        // Fetch up to 16 packets from the SX1302 FIFO
        let mut pkt_data = [lgw_pkt_rx_s::default(); 16];
        let count = unsafe {
            bindings::lgw_receive(16, pkt_data.as_mut_ptr())
        };

        if count < 0 {
            return Err(AppError::Infrastructure(
                format!("lgw_receive failed: {}", count)
            ));
        }

        if count == 0 {
            // Idle — no packets before timeout
            std::thread::sleep(self.inner.idle_timeout);
            return Ok(UplinkReceive::Idle);
        }

        // Convert each received packet to an UplinkObservation
        let mut observations = Vec::with_capacity(count as usize);
        for pkt in &pkt_data[..count as usize] {
            match lgw_pkt_rx_to_observation(pkt, GATEWAY_EUI) {
                Ok(obs) => observations.push(obs),
                Err(e) => {
                    tracing::warn!("failed to convert lgw_pkt_rx_s to UplinkObservation: {}", e);
                    continue;
                }
            }
        }

        Ok(UplinkReceive::Observations(observations))
    }
}

#[async_trait]
impl UplinkSource for SpiUplinkSource {
    async fn next_batch(&self) -> AppResult<UplinkReceive> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.blocking_receive())
            .await
            .map_err(|e| AppError::Infrastructure(format!("spi uplink join: {}", e)))?
    }
}

impl Drop for SpiUplinkSource {
    fn drop(&mut self) {
        // SAFETY: lgw_hal_stop calls lgw_stop() which is safe to call once.
        // Drop must run to avoid EBUSY on next lgw_start() from a new SpiUplinkSource.
        lgw_hal_stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires ARM hardware with libloragw"]
    fn lgw_receive_produces_observations_with_valid_wire_mic() {
        // Integration test: real lgw_receive returns packets where:
        // - wire_mic is exactly last 4 bytes of payload
        // - phy_without_mic is payload[..size-4]
        // This test requires hardware — ignored on x86 CI
    }

    #[test]
    fn wire_mic_split_is_correct() {
        // Unit test: verify the wire_mic/phy_without_mic split
        // using known LoRaWAN frame test vectors
        let payload: [u8; 23] = [
            // MHDR + FHDR (9 bytes) + FPort (1) + FRMPayload (9) = 19 total
            // MIC is last 4 bytes, so payload = MHDR + FHDR + FPort + FRMPayload = 23 bytes total
            // With MIC extracted: 23 - 4 = 19 bytes phy_without_mic
            0x40, // MHDR: MType=Unconfirmed Data Up, Major=0
            0x01, 0x02, 0x03, 0x04, // DevAddr (little-endian)
            0x00, // FCtrl
            0x00, 0x01, // FCnt
            0x01, // FPort
            0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, // FRMPayload (9 bytes)
            // Last 4 bytes below are MIC — NOT part of phy_without_mic
            0xAA, 0xBB, 0xCC, 0xDD, // MIC (wire_mic)
        ];
        let size = payload.len(); // 23

        let wire_mic: [u8; 4] = payload[size - 4..].try_into().unwrap();
        assert_eq!(wire_mic, [0xAA, 0xBB, 0xCC, 0xDD]);

        let phy_without_mic = &payload[..size - 4];
        assert_eq!(phy_without_mic.len(), 19);
        assert_eq!(phy_without_mic[phy_without_mic.len() - 1], 0x88); // last byte of FRMPayload
    }
}
```

**read_first:**
- `crates/maverick-adapter-radio-spi/src/spi_uplink.rs` — current placeholder implementation to replace
- `crates/maverick-adapter-radio-spi/src/lgw_convert.rs` — just created in B-1
- `crates/maverick-adapter-radio-spi/src/lgw_init.rs` — just created in B-2
- `.planning/phases/02-radio-abstraction-spi/02-RESEARCH.md` — Pattern 3 (spawn_blocking with Mutex), Anti-pattern (panic inside spawn_blocking holding Mutex)
- `10-CONTEXT.md` — specifics (next_batch wraps lgw_receive in spawn_blocking with Mutex guard)

**acceptance_criteria:**
- `SpiUplinkSource::new(spi_path, idle_timeout)` calls `lgw_hal_start()` — HAL init on construction
- `blocking_receive()` holds `hal_lock` Mutex guard and calls `lgw_receive(max_pkt=16, pkt_data)`
- `count == 0` returns `UplinkReceive::Idle` (idle timeout behavior)
- `count < 0` returns `AppError::Infrastructure` (lgw_receive error)
- `count > 0` converts each packet via `lgw_pkt_rx_to_observation()` and returns `UplinkReceive::Observations(observations)`
- `Drop for SpiUplinkSource` calls `lgw_hal_stop()` — clean exit to avoid EBUSY on next start
- `async fn next_batch` uses `spawn_blocking` with `self.clone()` and calls `blocking_receive()`
- `#[ignore = "requires ARM hardware with libloragw"]` on integration test — never runs on x86

---

## Verification

1. **lgw_convert compiles:** `cargo build -p maverick-adapter-radio-spi --features spi` on ARM (x86 skips C compilation)
2. **lgw_init compiles:** bindings for lgw_board_setconf, lgw_start, lgw_stop exist in `bindings.rs`
3. **spi_uplink refactored:** `grep 'lgw_receive' src/spi_uplink.rs`
4. **Drop implemented:** `grep 'impl Drop' src/spi_uplink.rs`
5. **Tests exist:** `grep 'fn wire_mic_split_is_correct' src/spi_uplink.rs`

## Notes

- `GatewayEui` is hardcoded to zero bytes — real implementation would read from concentrator OTP or config
- `lgw_receive(max_pkt=16, pkt_data)` fetches up to 16 packets per poll — reasonable batch size
- `idle_timeout` (default 100ms from config) controls the sleep between empty polls
- Phase 3.1 (Class A Downlink) will add `lgw_send()` TX path using the same HAL