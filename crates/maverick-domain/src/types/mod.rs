use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Eui64([u8; 8]);

impl Eui64 {
    pub fn new(bytes: [u8; 8]) -> Self {
        Self(bytes)
    }

    pub fn from_slice(slice: &[u8]) -> Option<Self> {
        if slice.len() == 8 {
            let mut bytes = [0u8; 8];
            bytes.copy_from_slice(slice);
            Some(Self(bytes))
        } else {
            None
        }
    }

    pub fn as_bytes(&self) -> [u8; 8] {
        self.0
    }

    pub fn as_bytes_slice(&self) -> &[u8; 8] {
        &self.0
    }
}

impl std::fmt::Display for Eui64 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, byte) in self.0.iter().enumerate() {
            if i > 0 {
                write!(f, ":")?;
            }
            write!(f, "{:02X}", byte)?;
        }
        Ok(())
    }
}

impl From<[u8; 8]> for Eui64 {
    fn from(bytes: [u8; 8]) -> Self {
        Self(bytes)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DevNonce(pub u16);

impl DevNonce {
    pub fn new(value: u16) -> Self {
        Self(value)
    }
}

impl From<u16> for DevNonce {
    fn from(value: u16) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameCounter(pub u32);

impl FrameCounter {
    pub fn new(value: u32) -> Self {
        Self(value)
    }

    pub fn increment(&mut self) {
        self.0 += 1;
    }
}

impl From<u32> for FrameCounter {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Frequency(pub u32);

impl Frequency {
    pub fn new(hz: u32) -> Self {
        Self(hz)
    }

    pub fn as_hz(&self) -> u32 {
        self.0
    }

    pub fn as_khz(&self) -> u32 {
        self.0 / 1000
    }

    pub fn as_mhz(&self) -> f64 {
        self.0 as f64 / 1_000_000.0
    }
}

impl From<u32> for Frequency {
    fn from(hz: u32) -> Self {
        Self(hz)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rssi(pub i16);

impl Rssi {
    pub fn new(value: i16) -> Self {
        Self(value)
    }

    pub fn as_i16(&self) -> i16 {
        self.0
    }
}

impl From<i16> for Rssi {
    fn from(value: i16) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Snr(pub f32);

impl Snr {
    pub fn new(value: f32) -> Self {
        Self(value)
    }

    pub fn as_f32(&self) -> f32 {
        self.0
    }
}

impl From<f32> for Snr {
    fn from(value: f32) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpreadingFactor(pub u8);

impl SpreadingFactor {
    pub fn new(sf: u8) -> Option<Self> {
        if sf >= 7 && sf <= 12 {
            Some(Self(sf))
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppKey(pub [u8; 16]);

impl AppKey {
    pub fn new(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> [u8; 16] {
        self.0
    }
}

impl From<[u8; 16]> for AppKey {
    fn from(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NwkKey(pub [u8; 16]);

impl NwkKey {
    pub fn new(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> [u8; 16] {
        self.0
    }
}

impl From<[u8; 16]> for NwkKey {
    fn from(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Region(pub u8);

impl Region {
    pub const EU868: Self = Self(0);
    pub const US915: Self = Self(1);
    pub const AU915: Self = Self(2);
    pub const AS923: Self = Self(3);

    pub fn code(&self) -> u8 {
        self.0
    }
}
