#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthcheckKind {
    Ping,
    Pong,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HealthcheckPacket {
    pub kind: HealthcheckKind,
    pub timestamp_nanos: u64,
}

const HEALTHCHECK_MAGIC: [u8; 4] = *b"TBDH";
const HEALTHCHECK_LENGTH: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthcheckError {
    BufferTooSmall,
    InvalidMagic,
    InvalidKind,
}

impl HealthcheckPacket {
    pub fn encode(self) -> [u8; HEALTHCHECK_LENGTH] {
        let mut buffer = [0_u8; HEALTHCHECK_LENGTH];
        buffer[0..4].copy_from_slice(&HEALTHCHECK_MAGIC);
        buffer[4] = match self.kind {
            HealthcheckKind::Ping => 1,
            HealthcheckKind::Pong => 2,
        };
        buffer[8..16].copy_from_slice(&self.timestamp_nanos.to_be_bytes());
        buffer
    }

    pub fn decode(buffer: &[u8]) -> Result<Self, HealthcheckError> {
        if buffer.len() < HEALTHCHECK_LENGTH {
            return Err(HealthcheckError::BufferTooSmall);
        }

        if buffer[0..4] != HEALTHCHECK_MAGIC {
            return Err(HealthcheckError::InvalidMagic);
        }

        let kind = match buffer[4] {
            1 => HealthcheckKind::Ping,
            2 => HealthcheckKind::Pong,
            _ => return Err(HealthcheckError::InvalidKind),
        };

        let timestamp_nanos = u64::from_be_bytes(buffer[8..16].try_into().unwrap());

        Ok(Self {
            kind,
            timestamp_nanos,
        })
    }

    pub fn is_healthcheck_packet(buffer: &[u8]) -> bool {
        buffer.len() >= HEALTHCHECK_LENGTH && buffer[0..4] == HEALTHCHECK_MAGIC
    }
}

#[cfg(test)]
mod tests {
    use super::{HealthcheckKind, HealthcheckPacket};

    #[test]
    fn round_trip_encode_decode() {
        let packet = HealthcheckPacket {
            kind: HealthcheckKind::Ping,
            timestamp_nanos: 1234,
        };

        let encoded = packet.encode();
        let decoded = HealthcheckPacket::decode(&encoded).expect("decode");
        assert_eq!(decoded, packet);
    }
}
