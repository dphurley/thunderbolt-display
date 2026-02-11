pub mod udp;

#[derive(Debug)]
pub enum TransportError {
    Io(std::io::Error),
}

impl From<std::io::Error> for TransportError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl std::fmt::Display for TransportError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransportError::Io(error) => write!(formatter, "transport io error: {error}"),
        }
    }
}

impl std::error::Error for TransportError {}

pub trait PacketSender {
    fn send(&mut self, packet: &[u8]) -> Result<usize, TransportError>;
}

pub trait PacketReceiver {
    fn receive(&mut self, buffer: &mut [u8]) -> Result<usize, TransportError>;
}
