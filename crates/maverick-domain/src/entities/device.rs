use serde::{Deserialize, Serialize};
use crate::types::{Eui64, AppKey, NwkKey, DevNonce, FrameCounter};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceClass {
    ClassA,
    ClassB,
    ClassC,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceState {
    Init,
    JoinPending,
    Active,
    Sleep,
    Dead,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceKeys {
    pub app_key: AppKey,
    pub nwk_key: NwkKey,
}

impl DeviceKeys {
    pub fn new(app_key: AppKey, nwk_key: NwkKey) -> Self {
        Self { app_key, nwk_key }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceSession {
    pub dev_addr: u32,
    pub app_s_key: [u8; 16],
    pub nwk_s_key: [u8; 16],
    pub frame_counter: FrameCounter,
    pub last_join_time: Option<i64>,
}

impl DeviceSession {
    pub fn new(dev_addr: u32, app_s_key: [u8; 16], nwk_s_key: [u8; 16]) -> Self {
        Self {
            dev_addr,
            app_s_key,
            nwk_s_key,
            frame_counter: FrameCounter::new(0),
            last_join_time: None,
        }
    }

    pub fn increment_fcnt(&mut self) {
        self.frame_counter.increment();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub dev_eui: Eui64,
    pub app_eui: Eui64,
    pub keys: DeviceKeys,
    pub session: Option<DeviceSession>,
    pub dev_nonce: Option<DevNonce>,
    pub class: DeviceClass,
    pub state: DeviceState,
    pub f_cnt_up: u32,
    pub f_cnt_down: u32,
}

impl Device {
    pub fn new(dev_eui: Eui64, app_eui: Eui64, keys: DeviceKeys) -> Self {
        Self {
            dev_eui,
            app_eui,
            keys,
            session: None,
            dev_nonce: None,
            class: DeviceClass::ClassA,
            state: DeviceState::Init,
            f_cnt_up: 0,
            f_cnt_down: 0,
        }
    }

    pub fn is_active(&self) -> bool {
        self.state == DeviceState::Active && self.session.is_some()
    }

    pub fn apply_session(&mut self, session: DeviceSession) {
        self.session = Some(session);
        self.state = DeviceState::Active;
    }
}