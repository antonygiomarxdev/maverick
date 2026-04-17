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
use std::sync::Mutex;

static HAL_INIT: Mutex<()> = Mutex::new(());

pub fn lgw_hal_start(spi_path: &str) -> AppResult<()> {
    let _guard = HAL_INIT
        .lock()
        .map_err(|_| AppError::Infrastructure("lgw hal mutex poisoned".to_string()))?;

    let mut board_conf = lgw_bindings::lgw_conf_board_s {
        lorawan_public: true,
        clksrc: 1,
        full_duplex: false,
        com_type: lgw_bindings::com_type_e_LGW_COM_SPI,
        com_path: [0; 64],
    };
    for (i, c) in spi_path.bytes().take(63).enumerate() {
        board_conf.com_path[i] = c;
    }
    let board_ptr = &mut board_conf as *mut _;
    let ret = unsafe { lgw_bindings::lgw_board_setconf(board_ptr) };
    if ret != 0 {
        return Err(AppError::Infrastructure(format!(
            "lgw_board_setconf failed: {}",
            ret
        )));
    }

    let mut rf0_conf = lgw_bindings::lgw_conf_rxrf_s {
        enable: true,
        freq_hz: 867_500_000,
        rssi_offset: -166.0,
        rssi_tcomp: lgw_bindings::lgw_rssi_tcomp_s {
            coeff_a: 0.0,
            coeff_b: 0.0,
            coeff_c: 0.0,
            coeff_d: 0.0,
            coeff_e: 0.0,
        },
        type_: 0, // lgw_radio_type_t_LGW_RADIO_TYPE_NONE
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
        freq_hz: 868_500_000,
        rssi_offset: -166.0,
        rssi_tcomp: lgw_bindings::lgw_rssi_tcomp_s {
            coeff_a: 0.0,
            coeff_b: 0.0,
            coeff_c: 0.0,
            coeff_d: 0.0,
            coeff_e: 0.0,
        },
        type_: 0, // lgw_radio_type_t_LGW_RADIO_TYPE_NONE
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
