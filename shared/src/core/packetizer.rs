use crate::core::packet::{VideoPacket, VideoPacketHeader};
use crate::core::sequence::SequenceNumber;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PacketizerConfig {
    pub max_payload_bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PacketizerError {
    EmptyPayload,
    PayloadTooLarge,
}

impl std::fmt::Display for PacketizerError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PacketizerError::EmptyPayload => write!(formatter, "packetizer payload is empty"),
            PacketizerError::PayloadTooLarge => {
                write!(formatter, "packetizer payload too large")
            }
        }
    }
}

impl std::error::Error for PacketizerError {}

pub struct Packetizer {
    config: PacketizerConfig,
    next_sequence_number: SequenceNumber,
}

impl Packetizer {
    pub fn new(config: PacketizerConfig, initial_sequence_number: SequenceNumber) -> Self {
        Self {
            config,
            next_sequence_number: initial_sequence_number,
        }
    }

    pub fn packetize(
        &mut self,
        frame_identifier: u32,
        timestamp_nanos: u64,
        payload: &[u8],
    ) -> Result<Vec<VideoPacket>, PacketizerError> {
        if payload.is_empty() {
            return Err(PacketizerError::EmptyPayload);
        }

        if self.config.max_payload_bytes == 0 {
            return Err(PacketizerError::PayloadTooLarge);
        }

        let chunks_total = ((payload.len() + self.config.max_payload_bytes - 1)
            / self.config.max_payload_bytes) as u16;

        let mut packets = Vec::with_capacity(chunks_total as usize);

        for (chunk_index, chunk) in payload
            .chunks(self.config.max_payload_bytes)
            .enumerate()
        {
            if chunk_index >= u16::MAX as usize {
                return Err(PacketizerError::PayloadTooLarge);
            }

            let header = VideoPacketHeader {
                sequence_number: self.next_sequence_number,
                timestamp_nanos,
                frame_identifier,
                chunk_index: chunk_index as u16,
                chunks_total,
            };
            self.next_sequence_number = self.next_sequence_number.next();

            packets.push(VideoPacket {
                header,
                payload: chunk.to_vec(),
            });
        }

        Ok(packets)
    }
}

#[cfg(test)]
mod tests {
    use super::{Packetizer, PacketizerConfig, PacketizerError};
    use crate::core::sequence::SequenceNumber;

    #[test]
    fn packetize_splits_payload() {
        let mut packetizer = Packetizer::new(
            PacketizerConfig {
                max_payload_bytes: 4,
            },
            SequenceNumber::new(1),
        );

        let payload = vec![1_u8, 2, 3, 4, 5, 6, 7];
        let packets = packetizer
            .packetize(9, 123, &payload)
            .expect("packetize failed");

        assert_eq!(packets.len(), 2);
        assert_eq!(packets[0].payload, vec![1, 2, 3, 4]);
        assert_eq!(packets[1].payload, vec![5, 6, 7]);
        assert_eq!(packets[0].header.chunk_index, 0);
        assert_eq!(packets[1].header.chunk_index, 1);
        assert_eq!(packets[0].header.chunks_total, 2);
    }

    #[test]
    fn packetize_rejects_empty_payload() {
        let mut packetizer = Packetizer::new(
            PacketizerConfig {
                max_payload_bytes: 4,
            },
            SequenceNumber::new(1),
        );

        let result = packetizer.packetize(1, 0, &[]);
        assert_eq!(result, Err(PacketizerError::EmptyPayload));
    }

    #[test]
    fn packetize_rejects_zero_mtu() {
        let mut packetizer = Packetizer::new(
            PacketizerConfig {
                max_payload_bytes: 0,
            },
            SequenceNumber::new(1),
        );

        let payload = vec![1_u8, 2, 3];
        let result = packetizer.packetize(1, 0, &payload);
        assert_eq!(result, Err(PacketizerError::PayloadTooLarge));
    }
}
