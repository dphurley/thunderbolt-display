use crate::transport::{PacketReceiver, PacketSender, TransportError};
use std::net::{SocketAddr, UdpSocket};
use std::time::Duration;

#[derive(Debug)]
pub struct UdpTransport {
    socket: UdpSocket,
}

impl UdpTransport {
    pub fn bind(local_addr: SocketAddr) -> Result<Self, TransportError> {
        let socket = UdpSocket::bind(local_addr)?;
        Ok(Self { socket })
    }

    pub fn connect(self, remote_addr: SocketAddr) -> Result<Self, TransportError> {
        self.socket.connect(remote_addr)?;
        Ok(self)
    }

    pub fn local_addr(&self) -> Result<SocketAddr, TransportError> {
        Ok(self.socket.local_addr()?)
    }

    pub fn set_read_timeout(&self, timeout: Option<Duration>) -> Result<(), TransportError> {
        self.socket.set_read_timeout(timeout)?;
        Ok(())
    }
}

impl PacketSender for UdpTransport {
    fn send(&mut self, packet: &[u8]) -> Result<usize, TransportError> {
        Ok(self.socket.send(packet)?)
    }
}

impl PacketReceiver for UdpTransport {
    fn receive(&mut self, buffer: &mut [u8]) -> Result<usize, TransportError> {
        Ok(self.socket.recv(buffer)?)
    }
}

#[cfg(test)]
mod tests {
    use super::UdpTransport;
    use crate::transport::{PacketReceiver, PacketSender};
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    #[test]
    #[ignore = "requires UDP socket access, enable explicitly when allowed"]
    fn udp_round_trip() {
        let local_sender = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
        let local_receiver = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);

        let sender = UdpTransport::bind(local_sender).expect("bind sender");
        let receiver = UdpTransport::bind(local_receiver).expect("bind receiver");

        let sender_address = sender.local_addr().unwrap();
        let receiver_address = receiver.local_addr().unwrap();

        let mut sender = sender.connect(receiver_address).expect("connect sender");
        let mut receiver = receiver.connect(sender_address).expect("connect receiver");

        let payload = b"hello";
        sender.send(payload).expect("send");

        let mut buffer = [0_u8; 64];
        let received = receiver.receive(&mut buffer).expect("receive");
        assert_eq!(&buffer[..received], payload);
    }
}
