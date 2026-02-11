pub mod types;
pub mod dummy;
#[cfg(target_os = "macos")]
pub mod macos;

use types::{DecodedFrame, EncodedFrame, RawFrame};

pub trait VideoEncoder {
    fn encode(&mut self, frame: &RawFrame) -> Result<EncodedFrame, CodecError>;
}

pub trait VideoDecoder {
    fn decode(&mut self, frame: &EncodedFrame) -> Result<DecodedFrame, CodecError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodecError {
    InvalidInput,
    Unsupported,
    InternalError,
}

impl std::fmt::Display for CodecError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CodecError::InvalidInput => write!(formatter, "codec invalid input"),
            CodecError::Unsupported => write!(formatter, "codec unsupported"),
            CodecError::InternalError => write!(formatter, "codec internal error"),
        }
    }
}

impl std::error::Error for CodecError {}
