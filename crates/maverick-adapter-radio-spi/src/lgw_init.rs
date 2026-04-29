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
