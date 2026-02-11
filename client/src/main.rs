use shared::codec::dummy::PassthroughCodec;
use shared::codec::types::EncodedFrame;
use shared::codec::VideoDecoder;
use shared::core::packet_codec::decode_packet;
use shared::core::reassembler::{FrameReassembler, ReassemblyError};
use shared::transport::udp::UdpTransport;
use shared::transport::PacketReceiver;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::{Duration, Instant};

#[cfg(target_os = "macos")]
use shared::codec::macos::h264::VideoToolboxH264Decoder;
#[cfg(target_os = "macos")]
use shared::platform::macos::network::detect_preferred_interface;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CodecChoice {
    Passthrough,
    H264,
}

#[derive(Debug)]
struct ClientConfig {
    bind_address: SocketAddr,
    remote_address: SocketAddr,
    max_packet_bytes: usize,
    max_in_flight_frames: usize,
    codec: CodecChoice,
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

    match config.codec {
        CodecChoice::Passthrough => {
            let mut decoder = PassthroughCodec::default();
            loop {
                if let Some(frame) = receive_frame(&mut receiver, &mut buffer, &mut reassembler, &mut packets_received) {
                    let encoded = EncodedFrame {
                        timestamp: Duration::from_nanos(frame.timestamp_nanos),
                        data: frame.payload,
                        is_keyframe: true,
                    };
                    let _ = decoder.decode(&encoded);
                    frames_received += 1;
                }

                report(&mut last_report, &mut frames_received, &mut packets_received);
            }
        }
        CodecChoice::H264 => {
            #[cfg(target_os = "macos")]
            {
                let mut decoder = VideoToolboxH264Decoder::new()?;
                loop {
                    if let Some(frame) = receive_frame(&mut receiver, &mut buffer, &mut reassembler, &mut packets_received) {
                        let encoded = EncodedFrame {
                            timestamp: Duration::from_nanos(frame.timestamp_nanos),
                            data: frame.payload,
                            is_keyframe: true,
                        };
                        let _ = decoder.decode(&encoded);
                        frames_received += 1;
                    }

                    report(&mut last_report, &mut frames_received, &mut packets_received);
                }
            }
            #[cfg(not(target_os = "macos"))]
            {
                return Err("H.264 codec is only supported on macOS".into());
            }
        }
    }
}

fn receive_frame(
    receiver: &mut UdpTransport,
    buffer: &mut [u8],
    reassembler: &mut FrameReassembler,
    packets_received: &mut u64,
) -> Option<shared::core::reassembler::ReassembledFrame> {
    match receiver.receive(buffer) {
        Ok(bytes_received) => {
            *packets_received += 1;
            let packet = decode_packet(&buffer[..bytes_received]).ok()?;
            match reassembler.push_packet(packet) {
                Ok(Some(frame)) => Some(frame),
                Ok(None) => None,
                Err(ReassemblyError::InvalidChunkIndex) => None,
                Err(ReassemblyError::InconsistentChunkCount) => None,
            }
        }
        Err(_) => None,
    }
}

fn report(last_report: &mut Instant, frames_received: &mut u64, packets_received: &mut u64) {
    if last_report.elapsed() >= Duration::from_secs(1) {
        eprintln!(
            "frames received: {frames_received}, packets received: {packets_received}"
        );
        *last_report = Instant::now();
        *frames_received = 0;
        *packets_received = 0;
    }
}

fn parse_args() -> Result<ClientConfig, String> {
    let mut bind_address: Option<SocketAddr> = None;
    let mut remote_address: Option<SocketAddr> = None;
    let mut max_packet_bytes: usize = 2048;
    let mut max_in_flight_frames: usize = 8;
    let mut auto_bind_port: Option<u16> = None;
    let mut codec = CodecChoice::Passthrough;

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
            "--auto-bind-port" => {
                let value = args.next().ok_or("missing --auto-bind-port value")?;
                auto_bind_port = Some(value.parse().map_err(|_| "invalid port")?);
            }
            "--codec" => {
                let value = args.next().ok_or("missing --codec value")?;
                codec = parse_codec(&value)?;
            }
            "--help" | "-h" => {
                return Err("".to_string());
            }
            _ => {
                return Err(format!("unknown argument: {argument}"));
            }
        }
    }

    if bind_address.is_none() {
        if let Some(port) = auto_bind_port {
            bind_address = auto_bind_socket(port)?;
        }
    }

    let bind_address = bind_address.ok_or("missing --bind (or use --auto-bind-port)")?;
    let remote_address = remote_address.ok_or("missing --remote")?;

    Ok(ClientConfig {
        bind_address,
        remote_address,
        max_packet_bytes,
        max_in_flight_frames,
        codec,
    })
}

fn parse_codec(value: &str) -> Result<CodecChoice, String> {
    match value {
        "passthrough" => Ok(CodecChoice::Passthrough),
        "h264" => Ok(CodecChoice::H264),
        _ => Err("invalid codec (use passthrough or h264)".to_string()),
    }
}

fn auto_bind_socket(port: u16) -> Result<Option<SocketAddr>, String> {
    #[cfg(target_os = "macos")]
    {
        if let Some(interface) = detect_preferred_interface() {
            eprintln!(
                "auto-bind selected interface {} with IPv4 {}",
                interface.name, interface.ipv4
            );
            return Ok(Some(SocketAddr::new(
                IpAddr::V4(interface.ipv4),
                port,
            )));
        }
    }

    let fallback = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), port);
    Ok(Some(fallback))
}

fn parse_socket_addr(value: &str) -> Result<SocketAddr, String> {
    value
        .parse()
        .map_err(|_| format!("invalid socket address: {value}"))
}

fn print_usage() {
    eprintln!(
        "usage: client --bind IP:PORT --remote IP:PORT [--max-packet-bytes N] [--max-in-flight-frames N] [--auto-bind-port PORT] [--codec passthrough|h264]"
    );
}
