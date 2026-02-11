use shared::core::packet_codec::decode_packet;
use shared::core::reassembler::{FrameReassembler, ReassemblyError};
use shared::transport::udp::UdpTransport;
use shared::transport::PacketReceiver;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

#[derive(Debug)]
struct ClientConfig {
    bind_address: SocketAddr,
    remote_address: SocketAddr,
    max_packet_bytes: usize,
    max_in_flight_frames: usize,
}

fn main() {
    let config = match parse_args() {
        Ok(config) => config,
        Err(message) => {
            eprintln!("{message}");
            print_usage();
            std::process::exit(1);
        }
    };

    if let Err(error) = run_client(config) {
        eprintln!("client error: {error}");
        std::process::exit(1);
    }
}

fn run_client(config: ClientConfig) -> Result<(), Box<dyn std::error::Error>> {
    let transport = UdpTransport::bind(config.bind_address)?.connect(config.remote_address)?;
    transport.set_read_timeout(Some(Duration::from_millis(250)))?;
    let mut receiver = transport;

    let mut reassembler = FrameReassembler::new(config.max_in_flight_frames);
    let mut buffer = vec![0_u8; config.max_packet_bytes];

    let mut last_report = Instant::now();
    let mut frames_received: u64 = 0;
    let mut packets_received: u64 = 0;

    loop {
        match receiver.receive(&mut buffer) {
            Ok(bytes_received) => {
                packets_received += 1;
                let packet = match decode_packet(&buffer[..bytes_received]) {
                    Ok(packet) => packet,
                    Err(_) => continue,
                };

                match reassembler.push_packet(packet) {
                    Ok(Some(frame)) => {
                        let _ = frame;
                        frames_received += 1;
                    }
                    Ok(None) => {}
                    Err(ReassemblyError::InvalidChunkIndex) => {}
                    Err(ReassemblyError::InconsistentChunkCount) => {}
                }
            }
            Err(_) => {}
        }

        if last_report.elapsed() >= Duration::from_secs(1) {
            eprintln!(
                "frames received: {frames_received}, packets received: {packets_received}"
            );
            last_report = Instant::now();
            frames_received = 0;
            packets_received = 0;
        }
    }
}

fn parse_args() -> Result<ClientConfig, String> {
    let mut bind_address: Option<SocketAddr> = None;
    let mut remote_address: Option<SocketAddr> = None;
    let mut max_packet_bytes: usize = 2048;
    let mut max_in_flight_frames: usize = 8;

    let mut args = std::env::args().skip(1);
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--bind" => {
                let value = args.next().ok_or("missing --bind value")?;
                bind_address = Some(parse_socket_addr(&value)?);
            }
            "--remote" => {
                let value = args.next().ok_or("missing --remote value")?;
                remote_address = Some(parse_socket_addr(&value)?);
            }
            "--max-packet-bytes" => {
                let value = args.next().ok_or("missing --max-packet-bytes value")?;
                max_packet_bytes = value
                    .parse()
                    .map_err(|_| "invalid max packet bytes")?;
            }
            "--max-in-flight-frames" => {
                let value = args.next().ok_or("missing --max-in-flight-frames value")?;
                max_in_flight_frames = value
                    .parse()
                    .map_err(|_| "invalid max in flight frames")?;
            }
            "--help" | "-h" => {
                return Err("".to_string());
            }
            _ => {
                return Err(format!("unknown argument: {argument}"));
            }
        }
    }

    let bind_address = bind_address.ok_or("missing --bind")?;
    let remote_address = remote_address.ok_or("missing --remote")?;

    Ok(ClientConfig {
        bind_address,
        remote_address,
        max_packet_bytes,
        max_in_flight_frames,
    })
}

fn parse_socket_addr(value: &str) -> Result<SocketAddr, String> {
    value
        .parse()
        .map_err(|_| format!("invalid socket address: {value}"))
}

fn print_usage() {
    eprintln!(
        "usage: client --bind IP:PORT --remote IP:PORT [--max-packet-bytes N] [--max-in-flight-frames N]"
    );
}
