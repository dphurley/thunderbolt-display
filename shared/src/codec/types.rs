use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    Rgba8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawFrame {
    pub width: u32,
    pub height: u32,
    pub pixel_format: PixelFormat,
    pub timestamp: Duration,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodedFrame {
    pub timestamp: Duration,
    pub data: Vec<u8>,
    pub is_keyframe: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedFrame {
    pub width: u32,
    pub height: u32,
    pub pixel_format: PixelFormat,
    pub timestamp: Duration,
    pub data: Vec<u8>,
}
