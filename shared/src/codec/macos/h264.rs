use crate::codec::types::{DecodedFrame, EncodedFrame, PixelFormat, RawFrame};
use crate::codec::{CodecError, VideoDecoder, VideoEncoder};

#[repr(C)]
struct VtH264EncodedFrame {
    data: *const u8,
    size: usize,
    is_keyframe: bool,
}

#[repr(C)]
struct VtH264DecodedFrame {
    data: *mut u8,
    size: usize,
    width: u32,
    height: u32,
}

#[repr(C)]
struct VtH264EncoderOpaque {
    _private: [u8; 0],
}
#[repr(C)]
struct VtH264DecoderOpaque {
    _private: [u8; 0],
}

#[link(name = "vtbridge", kind = "static")]
extern "C" {
    fn vt_h264_encoder_create(width: u32, height: u32, bitrate: u32, fps: u32) -> *mut VtH264EncoderOpaque;
    fn vt_h264_encoder_destroy(encoder: *mut VtH264EncoderOpaque);
    fn vt_h264_encoder_encode(
        encoder: *mut VtH264EncoderOpaque,
        rgba_data: *const u8,
        rgba_size: usize,
        out_frame: *mut VtH264EncodedFrame,
    ) -> bool;

    fn vt_h264_decoder_create() -> *mut VtH264DecoderOpaque;
    fn vt_h264_decoder_destroy(decoder: *mut VtH264DecoderOpaque);
    fn vt_h264_decoder_decode(
        decoder: *mut VtH264DecoderOpaque,
        data: *const u8,
        size: usize,
        out_frame: *mut VtH264DecodedFrame,
    ) -> bool;
}

#[derive(Debug)]
pub struct VideoToolboxH264Encoder {
    handle: *mut VtH264EncoderOpaque,
    width: u32,
    height: u32,
}

impl VideoToolboxH264Encoder {
    pub fn new(width: u32, height: u32, bitrate: u32, fps: u32) -> Result<Self, CodecError> {
        let handle = unsafe { vt_h264_encoder_create(width, height, bitrate, fps) };
        if handle.is_null() {
            return Err(CodecError::Unsupported);
        }
        Ok(Self { handle, width, height })
    }
}

impl Drop for VideoToolboxH264Encoder {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { vt_h264_encoder_destroy(self.handle) };
        }
    }
}

impl VideoEncoder for VideoToolboxH264Encoder {
    fn encode(&mut self, frame: &RawFrame) -> Result<EncodedFrame, CodecError> {
        if frame.width != self.width || frame.height != self.height {
            return Err(CodecError::InvalidInput);
        }
        if frame.pixel_format != PixelFormat::Rgba8 {
            return Err(CodecError::InvalidInput);
        }
        if frame.data.is_empty() {
            return Err(CodecError::InvalidInput);
        }

        let mut out = VtH264EncodedFrame {
            data: std::ptr::null(),
            size: 0,
            is_keyframe: false,
        };
        let ok = unsafe {
            vt_h264_encoder_encode(
                self.handle,
                frame.data.as_ptr(),
                frame.data.len(),
                &mut out,
            )
        };
        if !ok || out.data.is_null() || out.size == 0 {
            return Err(CodecError::InternalError);
        }

        let bytes = unsafe { std::slice::from_raw_parts(out.data, out.size) };
        Ok(EncodedFrame {
            timestamp: frame.timestamp,
            data: bytes.to_vec(),
            is_keyframe: out.is_keyframe,
        })
    }
}

#[derive(Debug)]
pub struct VideoToolboxH264Decoder {
    handle: *mut VtH264DecoderOpaque,
}

impl VideoToolboxH264Decoder {
    pub fn new() -> Result<Self, CodecError> {
        let handle = unsafe { vt_h264_decoder_create() };
        if handle.is_null() {
            return Err(CodecError::Unsupported);
        }
        Ok(Self { handle })
    }
}

impl Drop for VideoToolboxH264Decoder {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { vt_h264_decoder_destroy(self.handle) };
        }
    }
}

impl VideoDecoder for VideoToolboxH264Decoder {
    fn decode(&mut self, frame: &EncodedFrame) -> Result<DecodedFrame, CodecError> {
        if frame.data.is_empty() {
            return Err(CodecError::InvalidInput);
        }

        let mut out = VtH264DecodedFrame {
            data: std::ptr::null_mut(),
            size: 0,
            width: 0,
            height: 0,
        };

        let ok = unsafe { vt_h264_decoder_decode(self.handle, frame.data.as_ptr(), frame.data.len(), &mut out) };
        if !ok || out.data.is_null() || out.size == 0 {
            return Err(CodecError::InternalError);
        }

        let bytes = unsafe { std::slice::from_raw_parts(out.data, out.size) };
        Ok(DecodedFrame {
            width: out.width,
            height: out.height,
            pixel_format: PixelFormat::Bgra8,
            timestamp: frame.timestamp,
            data: bytes.to_vec(),
        })
    }
}
