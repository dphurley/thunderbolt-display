use crate::core::packet::VideoPacket;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReassembledFrame {
    pub frame_identifier: u32,
    pub timestamp_nanos: u64,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReassemblyError {
    InvalidChunkIndex,
    InconsistentChunkCount,
}

#[derive(Debug, Clone)]
struct FrameAssembly {
    timestamp_nanos: u64,
    chunks_total: u16,
    received_count: u16,
    chunks: Vec<Option<Vec<u8>>>,
}

#[derive(Debug, Clone)]
pub struct FrameReassembler {
    max_in_flight_frames: usize,
    frames: BTreeMap<u32, FrameAssembly>,
}

impl FrameReassembler {
    pub fn new(max_in_flight_frames: usize) -> Self {
        Self {
            max_in_flight_frames,
            frames: BTreeMap::new(),
        }
    }

    pub fn push_packet(
        &mut self,
        packet: VideoPacket,
    ) -> Result<Option<ReassembledFrame>, ReassemblyError> {
        if packet.header.chunk_index >= packet.header.chunks_total {
            return Err(ReassemblyError::InvalidChunkIndex);
        }

        let frame_identifier = packet.header.frame_identifier;

        let entry = self.frames.entry(frame_identifier).or_insert_with(|| {
            let chunks_total = packet.header.chunks_total;
            FrameAssembly {
                timestamp_nanos: packet.header.timestamp_nanos,
                chunks_total,
                received_count: 0,
                chunks: vec![None; chunks_total as usize],
            }
        });

        if entry.chunks_total != packet.header.chunks_total {
            return Err(ReassemblyError::InconsistentChunkCount);
        }

        let chunk_index = packet.header.chunk_index as usize;
        if entry.chunks[chunk_index].is_none() {
            entry.chunks[chunk_index] = Some(packet.payload);
            entry.received_count += 1;
        }

        if entry.received_count == entry.chunks_total {
            let mut payload = Vec::new();
            for chunk in entry.chunks.iter() {
                if let Some(bytes) = chunk {
                    payload.extend_from_slice(bytes);
                }
            }

            let frame = ReassembledFrame {
                frame_identifier,
                timestamp_nanos: entry.timestamp_nanos,
                payload,
            };
            self.frames.remove(&frame_identifier);
            return Ok(Some(frame));
        }

        self.evict_if_needed();
        Ok(None)
    }

    fn evict_if_needed(&mut self) {
        while self.frames.len() > self.max_in_flight_frames {
            if let Some(oldest_key) = self.frames.keys().next().cloned() {
                self.frames.remove(&oldest_key);
            } else {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{FrameReassembler, ReassemblyError};
    use crate::core::packet::{VideoPacket, VideoPacketHeader};
    use crate::core::sequence::SequenceNumber;

    fn packet(
        frame_identifier: u32,
        chunk_index: u16,
        chunks_total: u16,
        payload: &[u8],
    ) -> VideoPacket {
        VideoPacket {
            header: VideoPacketHeader {
                sequence_number: SequenceNumber::new(1),
                timestamp_nanos: 10,
                frame_identifier,
                chunk_index,
                chunks_total,
            },
            payload: payload.to_vec(),
        }
    }

    #[test]
    fn reassembles_in_order_chunks() {
        let mut reassembler = FrameReassembler::new(4);

        let first = packet(7, 0, 2, b"hello ");
        let second = packet(7, 1, 2, b"world");

        assert!(reassembler.push_packet(first).unwrap().is_none());
        let completed = reassembler.push_packet(second).unwrap();
        let frame = completed.expect("frame should be complete");

        assert_eq!(frame.payload, b"hello world");
        assert_eq!(frame.frame_identifier, 7);
    }

    #[test]
    fn reassembles_out_of_order_chunks() {
        let mut reassembler = FrameReassembler::new(4);

        let first = packet(7, 1, 2, b"world");
        let second = packet(7, 0, 2, b"hello ");

        assert!(reassembler.push_packet(first).unwrap().is_none());
        let completed = reassembler.push_packet(second).unwrap();
        let frame = completed.expect("frame should be complete");

        assert_eq!(frame.payload, b"hello world");
    }

    #[test]
    fn ignores_duplicate_chunks() {
        let mut reassembler = FrameReassembler::new(4);

        let first = packet(7, 0, 2, b"hello ");
        let duplicate = packet(7, 0, 2, b"hello ");
        let second = packet(7, 1, 2, b"world");

        assert!(reassembler.push_packet(first).unwrap().is_none());
        assert!(reassembler.push_packet(duplicate).unwrap().is_none());
        let completed = reassembler.push_packet(second).unwrap();
        let frame = completed.expect("frame should be complete");

        assert_eq!(frame.payload, b"hello world");
    }

    #[test]
    fn rejects_invalid_chunk_index() {
        let mut reassembler = FrameReassembler::new(4);
        let bad_packet = packet(7, 2, 2, b"oops");

        let result = reassembler.push_packet(bad_packet);
        assert_eq!(result, Err(ReassemblyError::InvalidChunkIndex));
    }

    #[test]
    fn rejects_inconsistent_chunk_counts() {
        let mut reassembler = FrameReassembler::new(4);
        let first = packet(7, 0, 2, b"hello ");
        let second = packet(7, 1, 3, b"world");

        assert!(reassembler.push_packet(first).unwrap().is_none());
        let result = reassembler.push_packet(second);
        assert_eq!(result, Err(ReassemblyError::InconsistentChunkCount));
    }
}
