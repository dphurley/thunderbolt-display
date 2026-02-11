use crate::codec::types::{DecodedFrame, EncodedFrame, PixelFormat, RawFrame};
use crate::codec::{CodecError, VideoDecoder, VideoEncoder};

#[derive(Debug, Default)]
pub struct PassthroughCodec;

impl VideoEncoder for PassthroughCodec {
    fn encode(&mut self, frame: &RawFrame) -> Result<EncodedFrame, CodecError> {
        if frame.data.is_empty() {
            return Err(CodecError::InvalidInput);
        }

        Ok(EncodedFrame {
            timestamp: frame.timestamp,
            data: frame.data.clone(),
            is_keyframe: true,
        })
    }
}

impl VideoDecoder for PassthroughCodec {
    fn decode(&mut self, frame: &EncodedFrame) -> Result<DecodedFrame, CodecError> {
        if frame.data.is_empty() {
            return Err(CodecError::InvalidInput);
        }

        Ok(DecodedFrame {
            width: 1,
            height: 1,
            pixel_format: PixelFormat::Rgba8,
            timestamp: frame.timestamp,
            data: frame.data.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::PassthroughCodec;
    use crate::codec::types::{PixelFormat, RawFrame};
    use crate::codec::{VideoDecoder, VideoEncoder};
    use std::time::Duration;

    #[test]
    fn encode_decode_round_trip() {
        let mut codec = PassthroughCodec::default();
        let frame = RawFrame {
            width: 1,
            height: 1,
            pixel_format: PixelFormat::Rgba8,
            timestamp: Duration::from_millis(5),
            data: vec![1, 2, 3, 4],
        };

        let encoded = codec.encode(&frame).expect("encode");
        let decoded = codec.decode(&encoded).expect("decode");

        assert_eq!(decoded.data, frame.data);
        assert_eq!(decoded.timestamp, frame.timestamp);
    }
}
