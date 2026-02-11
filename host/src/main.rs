use shared::codec::dummy::PassthroughCodec;
use shared::codec::types::{PixelFormat, RawFrame};
use shared::codec::VideoEncoder;
use shared::core::packet_codec::encode_packet;
use shared::core::packetizer::{Packetizer, PacketizerConfig};
use shared::core::sequence::SequenceNumber;
use shared::transport::udp::UdpTransport;
use shared::transport::PacketSender;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::{Duration, Instant};

#[cfg(target_os = "macos")]
use shared::platform::macos::network::detect_preferred_interface;

#[derive(Debug)]
struct HostConfig {
    bind_address: SocketAddr,
    remote_address: SocketAddr,
    payload_bytes: usize,
    max_payload_bytes: usize,
    frame_interval: Duration,
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

    if let Err(error) = run_host(config) {
        eprintln!("host error: {error}");
        std::process::exit(1);
    }
}

fn run_host(config: HostConfig) -> Result<(), Box<dyn std::error::Error>> {
    let transport = UdpTransport::bind(config.bind_address)?.connect(config.remote_address)?;
    let mut sender = transport;

    let mut encoder = PassthroughCodec::default();
    let mut packetizer = Packetizer::new(
        PacketizerConfig {
            max_payload_bytes: config.max_payload_bytes,
        },
        SequenceNumber::new(0),
    );

    let mut frame_identifier: u32 = 0;
    let mut last_report = Instant::now();
    let mut frames_sent: u64 = 0;

    loop {
        let timestamp_nanos = current_time_nanos();
        let raw_frame = RawFrame {
            width: 1,
            height: 1,
            pixel_format: PixelFormat::Rgba8,
            timestamp: Duration::from_nanos(timestamp_nanos),
            data: vec![0xAB; config.payload_bytes],
        };

        let encoded = encoder.encode(&raw_frame)?;
        let payload = encoded.data;

        let packets = packetizer.packetize(frame_identifier, timestamp_nanos, &payload)?;
        for packet in packets {
            let buffer = encode_packet(&packet);
            sender.send(&buffer)?;
        }

        frames_sent += 1;
        frame_identifier = frame_identifier.wrapping_add(1);

        if last_report.elapsed() >= Duration::from_secs(1) {
            eprintln!("frames sent: {frames_sent}");
            last_report = Instant::now();
            frames_sent = 0;
        }

        std::thread::sleep(config.frame_interval);
    }
}

fn current_time_nanos() -> u64 {
    let now = std::time::SystemTime::now();
    let duration = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0));
    duration.as_nanos() as u64
}

fn parse_args() -> Result<HostConfig, String> {
    let mut bind_address: Option<SocketAddr> = None;
    let mut remote_address: Option<SocketAddr> = None;
    let mut payload_bytes: usize = 1024;
    let mut max_payload_bytes: usize = 1200;
    let mut frame_interval = Duration::from_millis(16);
    let mut auto_bind_port: Option<u16> = None;

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
            "--payload-bytes" => {
                let value = args.next().ok_or("missing --payload-bytes value")?;
                payload_bytes = value.parse().map_err(|_| "invalid payload bytes")?;
            }
            "--max-payload-bytes" => {
                let value = args.next().ok_or("missing --max-payload-bytes value")?;
                max_payload_bytes = value
                    .parse()
                    .map_err(|_| "invalid max payload bytes")?;
            }
            "--frame-interval-ms" => {
                let value = args.next().ok_or("missing --frame-interval-ms value")?;
                let millis: u64 = value.parse().map_err(|_| "invalid frame interval")?;
                frame_interval = Duration::from_millis(millis);
            }
            "--auto-bind-port" => {
                let value = args.next().ok_or("missing --auto-bind-port value")?;
                auto_bind_port = Some(value.parse().map_err(|_| "invalid port")?);
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

    Ok(HostConfig {
        bind_address,
        remote_address,
        payload_bytes,
        max_payload_bytes,
        frame_interval,
    })
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
        "usage: host --bind IP:PORT --remote IP:PORT [--payload-bytes N] [--max-payload-bytes N] [--frame-interval-ms N] [--auto-bind-port PORT]"
    );
}
