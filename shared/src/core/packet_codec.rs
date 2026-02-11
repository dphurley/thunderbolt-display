use crate::core::packet::{VideoPacket, VideoPacketHeader, VIDEO_PACKET_HEADER_LENGTH};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PacketCodecError {
    BufferTooSmall,
}

pub fn encode_packet(packet: &VideoPacket) -> Vec<u8> {
    let mut buffer = vec![0_u8; VIDEO_PACKET_HEADER_LENGTH + packet.payload.len()];
    packet
        .header
        .encode(&mut buffer[..VIDEO_PACKET_HEADER_LENGTH])
        .expect("header buffer is sized correctly");
    buffer[VIDEO_PACKET_HEADER_LENGTH..].copy_from_slice(&packet.payload);
    buffer
}

pub fn decode_packet(buffer: &[u8]) -> Result<VideoPacket, PacketCodecError> {
    if buffer.len() < VIDEO_PACKET_HEADER_LENGTH {
        return Err(PacketCodecError::BufferTooSmall);
    }

    let header = VideoPacketHeader::decode(&buffer[..VIDEO_PACKET_HEADER_LENGTH])
        .map_err(|_| PacketCodecError::BufferTooSmall)?;
    let payload = buffer[VIDEO_PACKET_HEADER_LENGTH..].to_vec();

    Ok(VideoPacket { header, payload })
}

#[cfg(test)]
mod tests {
    use super::{decode_packet, encode_packet, PacketCodecError};
    use crate::core::packet::VideoPacketHeader;
    use crate::core::sequence::SequenceNumber;
    use crate::core::packet::VideoPacket;

    #[test]
    fn round_trip_packet_encode_decode() {
        let packet = VideoPacket {
            header: VideoPacketHeader {
                sequence_number: SequenceNumber::new(9),
                timestamp_nanos: 111,
                frame_identifier: 7,
                chunk_index: 0,
                chunks_total: 1,
            },
            payload: b"payload".to_vec(),
        };

        let buffer = encode_packet(&packet);
        let decoded = decode_packet(&buffer).expect("decode");

        assert_eq!(decoded, packet);
    }

    #[test]
    fn decode_fails_on_small_buffer() {
        let buffer = vec![0_u8; 3];
        let result = decode_packet(&buffer);
        assert_eq!(result, Err(PacketCodecError::BufferTooSmall));
    }
}
