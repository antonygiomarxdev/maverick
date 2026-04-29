//! libloragw HAL initialization and cleanup.
//!
//! lgw_start() requires prior lgw_board_setconf() with board configuration.
//! lgw_stop() must be called on Drop to avoid EBUSY on next lgw_start().
//!
//! RAK2287 (SX1302) uses sensible defaults for:
//!   - Clock source (SX1250/SX1257)
//!   - Full duplex configuration
//!   - RF chain 0 and 1 configuration with correct frequency plans

use crate::lgw_bindings;
use maverick_core::error::{AppError, AppResult};
use std::process::Command;
use std::sync::Mutex;

static HAL_INIT: Mutex<()> = Mutex::new(());

/// Reset the SX1302 concentrator via GPIO before initialization.
///
/// The Semtech reference packet forwarder runs `reset_lgw.sh start` before
/// `lgw_start()`. Without this hard reset, the SX1302/SX1250 may be left in
/// a bad state from a previous run and SPI communication with the radios will
/// fail (typically "Failed to set SX1250_0 in STANDBY_RC mode").
fn reset_concentrator() {
    const RESET_SCRIPTS: &[&str] = &[
        "/usr/local/bin/maverick-reset-spi.sh",
        "/usr/local/bin/reset_lgw.sh",
    ];

    for script in RESET_SCRIPTS {
        if std::path::Path::new(script).exists() {
            tracing::info!("Resetting concentrator via {}", script);
            match Command::new(script).arg("start").output() {
                Ok(output) => {
                    if output.status.success() {
                        tracing::info!("Concentrator reset succeeded");
                        return;
                    } else {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        tracing::warn!(
                            "Concentrator reset script {} exited with status {:?}: {}",
                            script,
                            output.status.code(),
                            stderr.trim()
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to run concentrator reset script {}: {}", script, e);
                }
            }
        }
    }

    tracing::debug!("No concentrator reset script found; skipping GPIO reset");
}

pub fn lgw_hal_start(spi_path: &str) -> AppResult<()> {
    let _guard = HAL_INIT
        .lock()
        .map_err(|_| AppError::Infrastructure("lgw hal mutex poisoned".to_string()))?;

    reset_concentrator();

    let mut board_conf = lgw_bindings::lgw_conf_board_s {
        lorawan_public: true,
        clksrc: 0,
        full_duplex: false,
        com_type: lgw_bindings::com_type_e_LGW_COM_SPI,
        com_path: [0; 64],
    };
    for (i, c) in spi_path.bytes().take(63).enumerate() {
        board_conf.com_path[i] = c as std::os::raw::c_char;
    }
    let board_ptr = &mut board_conf as *mut _;
    let ret = unsafe { lgw_bindings::lgw_board_setconf(board_ptr) };
    if ret != 0 {
        return Err(AppError::Infrastructure(format!(
            "lgw_board_setconf failed: {}",
            ret
        )));
    }

    // AU915 defaults for RAK2287 (SX1302 + SX1250)
    let mut rf0_conf = lgw_bindings::lgw_conf_rxrf_s {
        enable: true,
        freq_hz: 915_900_000,
        rssi_offset: -161.0,
        rssi_tcomp: lgw_bindings::lgw_rssi_tcomp_s {
            coeff_a: 0.0,
            coeff_b: 0.0,
            coeff_c: 0.0,
            coeff_d: 0.0,
            coeff_e: 0.0,
        },
        type_: lgw_bindings::lgw_radio_type_t_LGW_RADIO_TYPE_SX1250,
        tx_enable: false,
        single_input_mode: false,
    };
    let rf0_ptr = &mut rf0_conf as *mut _;
    let ret = unsafe { lgw_bindings::lgw_rxrf_setconf(0, rf0_ptr) };
    if ret != 0 {
        return Err(AppError::Infrastructure(format!(
            "lgw_rxrf_setconf chain 0 failed: {}",
            ret
        )));
    }

    let mut rf1_conf = lgw_bindings::lgw_conf_rxrf_s {
        enable: true,
        freq_hz: 917_500_000,
        rssi_offset: -161.0,
        rssi_tcomp: lgw_bindings::lgw_rssi_tcomp_s {
            coeff_a: 0.0,
            coeff_b: 0.0,
            coeff_c: 0.0,
            coeff_d: 0.0,
            coeff_e: 0.0,
        },
        type_: lgw_bindings::lgw_radio_type_t_LGW_RADIO_TYPE_SX1250,
        tx_enable: false,
        single_input_mode: false,
    };
    let rf1_ptr = &mut rf1_conf as *mut _;
    let ret = unsafe { lgw_bindings::lgw_rxrf_setconf(1, rf1_ptr) };
    if ret != 0 {
        return Err(AppError::Infrastructure(format!(
            "lgw_rxrf_setconf chain 1 failed: {}",
            ret
        )));
    }

    // Configure 8 multi-SF LoRa channels (AU915-like plan)
    // RF0=915.9 MHz center, RF1=917.5 MHz center
    let if_freqs: [(i32, u8); 8] = [
        (-400_000, 1), // 917.1 MHz on RF1
        (-200_000, 1), // 917.3 MHz on RF1
        (0, 1),        // 917.5 MHz on RF1
        (-400_000, 0), // 915.5 MHz on RF0
        (-200_000, 0), // 915.7 MHz on RF0
        (0, 0),        // 915.9 MHz on RF0
        (200_000, 0),  // 916.1 MHz on RF0
        (400_000, 0),  // 916.3 MHz on RF0
    ];
    for (i, &(freq_hz, rf_chain)) in if_freqs.iter().enumerate() {
        let mut if_conf = lgw_bindings::lgw_conf_rxif_s {
            enable: true,
            rf_chain,
            freq_hz,
            bandwidth: 0,      // default
            datarate: 7,       // DR_LORA_SF7
            sync_word_size: 0, // default
            sync_word: 0,
            implicit_hdr: false,
            implicit_payload_length: 0,
            implicit_crc_en: false,
            implicit_coderate: 0,
        };
        let if_ptr = &mut if_conf as *mut _;
        let ret = unsafe { lgw_bindings::lgw_rxif_setconf(i as u8, if_ptr) };
        if ret != 0 {
            return Err(AppError::Infrastructure(format!(
                "lgw_rxif_setconf channel {} failed: {}",
                i, ret
            )));
        }
    }

    // Configure LoRa service channel (channel 8)
    let mut service_conf = lgw_bindings::lgw_conf_rxif_s {
        enable: true,
        rf_chain: 1,
        freq_hz: -200_000, // 917.3 MHz on RF1
        bandwidth: 0x05,   // BW_250KHZ
        datarate: 7,       // DR_LORA_SF7
        sync_word_size: 0,
        sync_word: 0,
        implicit_hdr: false,
        implicit_payload_length: 0,
        implicit_crc_en: false,
        implicit_coderate: 0,
    };
    let service_ptr = &mut service_conf as *mut _;
    let ret = unsafe { lgw_bindings::lgw_rxif_setconf(8, service_ptr) };
    if ret != 0 {
        return Err(AppError::Infrastructure(format!(
            "lgw_rxif_setconf service channel failed: {}",
            ret
        )));
    }

    // Configure demodulator
    let mut demod_conf = lgw_bindings::lgw_conf_demod_s {
        multisf_datarate: 0xFF, // enable all SFs
    };
    let demod_ptr = &mut demod_conf as *mut _;
    let ret = unsafe { lgw_bindings::lgw_demod_setconf(demod_ptr) };
    if ret != 0 {
        return Err(AppError::Infrastructure(format!(
            "lgw_demod_setconf failed: {}",
            ret
        )));
    }

    let ret = unsafe { lgw_bindings::lgw_start() };
    if ret != 0 {
        return Err(AppError::Infrastructure(format!(
            "lgw_start failed: {} (device busy or permissions?)",
            ret
        )));
    }

    tracing::info!("libloragw HAL started on {}", spi_path);
    Ok(())
}

pub fn lgw_hal_stop() {
    if let Ok(_guard) = HAL_INIT.lock() {
        let ret = unsafe { lgw_bindings::lgw_stop() };
        if ret == 0 {
            tracing::info!("libloragw HAL stopped");
        } else {
            tracing::warn!("lgw_stop returned {} (may be already stopped)", ret);
        }
    }
}
