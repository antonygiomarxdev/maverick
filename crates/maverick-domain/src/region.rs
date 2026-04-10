use thiserror::Error;

/// Supported regional plans for v1 baseline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RegionId {
    Eu868,
    Us915,
    Au915,
    As923,
    Eu433,
}

#[derive(Debug, Error, PartialEq, Eq)]
#[error("unknown region id: {0}")]
pub struct UnknownRegionError(pub String);

impl std::str::FromStr for RegionId {
    type Err = UnknownRegionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_uppercase().as_str() {
            "EU868" => Ok(Self::Eu868),
            "US915" => Ok(Self::Us915),
            "AU915" => Ok(Self::Au915),
            "AS923" => Ok(Self::As923),
            "EU433" => Ok(Self::Eu433),
            other => Err(UnknownRegionError(other.to_string())),
        }
    }
}

impl std::fmt::Display for RegionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Eu868 => "EU868",
            Self::Us915 => "US915",
            Self::Au915 => "AU915",
            Self::As923 => "AS923",
            Self::Eu433 => "EU433",
        };
        f.write_str(s)
    }
}
