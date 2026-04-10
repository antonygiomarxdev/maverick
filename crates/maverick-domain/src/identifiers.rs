use thiserror::Error;

/// 8-byte EUI as fixed array (network byte order as stored).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Eui64(pub [u8; 8]);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DevEui(pub Eui64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GatewayEui(pub Eui64);

/// 32-bit device address after join (LoRaWAN dev_addr).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DevAddr(pub u32);

#[derive(Debug, Error, PartialEq, Eq)]
#[error("invalid hex length for EUI64")]
pub struct InvalidEuiHex;

impl DevEui {
    /// Parse 16 hex chars (big-endian byte order).
    pub fn from_hex(s: &str) -> Result<Self, InvalidEuiHex> {
        Eui64::from_hex(s).map(DevEui)
    }
}

impl GatewayEui {
    pub fn from_hex(s: &str) -> Result<Self, InvalidEuiHex> {
        Eui64::from_hex(s).map(GatewayEui)
    }
}

impl Eui64 {
    pub fn from_hex(s: &str) -> Result<Self, InvalidEuiHex> {
        let s = s.trim();
        if s.len() != 16 {
            return Err(InvalidEuiHex);
        }
        let mut out = [0u8; 8];
        for (i, chunk) in s.as_bytes().chunks(2).enumerate() {
            if chunk.len() != 2 {
                return Err(InvalidEuiHex);
            }
            let hi = hex_nibble(chunk[0])?;
            let lo = hex_nibble(chunk[1])?;
            out[i] = (hi << 4) | lo;
        }
        Ok(Eui64(out))
    }
}

fn hex_nibble(b: u8) -> Result<u8, InvalidEuiHex> {
    match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'a'..=b'f' => Ok(b - b'a' + 10),
        b'A'..=b'F' => Ok(b - b'A' + 10),
        _ => Err(InvalidEuiHex),
    }
}
