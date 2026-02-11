use crate::codec::types::{DecodedFrame, EncodedFrame, RawFrame};
use crate::codec::{CodecError, VideoDecoder, VideoEncoder};

#[derive(Debug, Default)]
pub struct VideoToolboxH264Encoder;

#[derive(Debug, Default)]
pub struct VideoToolboxH264Decoder;

impl VideoEncoder for VideoToolboxH264Encoder {
    fn encode(&mut self, _frame: &RawFrame) -> Result<EncodedFrame, CodecError> {
        Err(CodecError::Unsupported)
    }
}

impl VideoDecoder for VideoToolboxH264Decoder {
    fn decode(&mut self, _frame: &EncodedFrame) -> Result<DecodedFrame, CodecError> {
        Err(CodecError::Unsupported)
    }
}
