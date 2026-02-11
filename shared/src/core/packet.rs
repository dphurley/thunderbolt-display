use crate::core::sequence::SequenceNumber;

pub const VIDEO_PACKET_HEADER_LENGTH: usize = 4 + 8 + 4 + 2 + 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VideoPacketHeader {
    pub sequence_number: SequenceNumber,
    pub timestamp_nanos: u64,
    pub frame_identifier: u32,
    pub chunk_index: u16,
    pub chunks_total: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoPacket {
    pub header: VideoPacketHeader,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PacketDecodeError {
    BufferTooSmall,
}

impl VideoPacketHeader {
    pub fn encode(self, buffer: &mut [u8]) -> Result<(), PacketDecodeError> {
        if buffer.len() < VIDEO_PACKET_HEADER_LENGTH {
            return Err(PacketDecodeError::BufferTooSmall);
        }

        buffer[0..4].copy_from_slice(&self.sequence_number.value().to_be_bytes());
        buffer[4..12].copy_from_slice(&self.timestamp_nanos.to_be_bytes());
        buffer[12..16].copy_from_slice(&self.frame_identifier.to_be_bytes());
        buffer[16..18].copy_from_slice(&self.chunk_index.to_be_bytes());
        buffer[18..20].copy_from_slice(&self.chunks_total.to_be_bytes());

        Ok(())
    }

    pub fn decode(buffer: &[u8]) -> Result<Self, PacketDecodeError> {
        if buffer.len() < VIDEO_PACKET_HEADER_LENGTH {
            return Err(PacketDecodeError::BufferTooSmall);
        }

        let sequence_number = u32::from_be_bytes(buffer[0..4].try_into().unwrap());
        let timestamp_nanos = u64::from_be_bytes(buffer[4..12].try_into().unwrap());
        let frame_identifier = u32::from_be_bytes(buffer[12..16].try_into().unwrap());
        let chunk_index = u16::from_be_bytes(buffer[16..18].try_into().unwrap());
        let chunks_total = u16::from_be_bytes(buffer[18..20].try_into().unwrap());

        Ok(Self {
            sequence_number: SequenceNumber::new(sequence_number),
            timestamp_nanos,
            frame_identifier,
            chunk_index,
            chunks_total,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{PacketDecodeError, VideoPacketHeader, VIDEO_PACKET_HEADER_LENGTH};
    use crate::core::sequence::SequenceNumber;

    #[test]
    fn round_trip_header_encode_decode() {
        let header = VideoPacketHeader {
            sequence_number: SequenceNumber::new(42),
            timestamp_nanos: 123456789,
            frame_identifier: 7,
            chunk_index: 1,
            chunks_total: 3,
        };

        let mut buffer = vec![0_u8; VIDEO_PACKET_HEADER_LENGTH];
        header.encode(&mut buffer).unwrap();

        let decoded = VideoPacketHeader::decode(&buffer).unwrap();
        assert_eq!(decoded, header);
    }

    #[test]
    fn encode_fails_on_small_buffer() {
        let header = VideoPacketHeader {
            sequence_number: SequenceNumber::new(1),
            timestamp_nanos: 0,
            frame_identifier: 0,
            chunk_index: 0,
            chunks_total: 1,
        };

        let mut buffer = vec![0_u8; VIDEO_PACKET_HEADER_LENGTH - 1];
        let result = header.encode(&mut buffer);
        assert_eq!(result, Err(PacketDecodeError::BufferTooSmall));
    }

    #[test]
    fn decode_fails_on_small_buffer() {
        let buffer = vec![0_u8; VIDEO_PACKET_HEADER_LENGTH - 1];
        let result = VideoPacketHeader::decode(&buffer);
        assert_eq!(result, Err(PacketDecodeError::BufferTooSmall));
    }
}
