use thiserror::Error;

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("Invalid key length: expected {expected} bytes, got {got}")]
    InvalidKeyLength { expected: usize, got: usize },

    #[error("Invalid IV length: expected {expected} bytes, got {got}")]
    InvalidIvLength { expected: usize, got: usize },

    #[error("Invalid MIC size: expected {expected} bytes, got {got}")]
    InvalidMicSize { expected: usize, got: usize },

    #[error("Invalid payload size: must be at least {min} bytes, got {got}")]
    InvalidPayloadSize { min: usize, got: usize },

    #[error("Invalid frame header: insufficient bytes ({got}) for MHDR(1) + DevAddr(4)")]
    InvalidFrameHeader { got: usize },

    #[error("CMAC computation failed: {context}")]
    CmacComputation { context: String },

    #[error("AES encryption/decryption failed: {context}")]
    AesOperation { context: String },

    #[error("MIC mismatch: expected {expected:?}, computed {computed:?}")]
    MicMismatch {
        expected: [u8; 4],
        computed: [u8; 4],
    },

    #[error("Invalid DevAddr in frame: {value:#010X}")]
    InvalidDevAddr { value: u32 },

    #[error("Invalid FPort: value {value} is reserved for MAC commands")]
    ReservedFPort { value: u8 },

    #[error("Frame counter too old: FCnt {fcnt} < expected minimum {min_fcnt}")]
    FrameCounterTooOld { fcnt: u32, min_fcnt: u32 },

    #[error("Zero frame counter increment detected (replay attack?)")]
    ZeroIncrement,

    #[error("Buffer overflow: requested {requested} bytes, available {available}")]
    BufferOverflow { requested: usize, available: usize },

    #[error("Internal cryptographic library error: {0}")]
    InternalError(&'static str),
}

impl CryptoError {
    pub fn invalid_key_length(expected: usize, got: usize) -> Self {
        Self::InvalidKeyLength { expected, got }
    }

    pub fn invalid_iv_length(expected: usize, got: usize) -> Self {
        Self::InvalidIvLength { expected, got }
    }

    pub fn mic_mismatch(expected: [u8; 4], computed: [u8; 4]) -> Self {
        Self::MicMismatch { expected, computed }
    }

    pub fn cmac_failure(context: impl Into<String>) -> Self {
        Self::CmacComputation {
            context: context.into(),
        }
    }

    pub fn aes_failure(context: impl Into<String>) -> Self {
        Self::AesOperation {
            context: context.into(),
        }
    }
}

pub type Result<T> = std::result::Result<T, CryptoError>;
