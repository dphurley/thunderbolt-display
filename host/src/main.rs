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
use shared::codec::macos::h264::VideoToolboxH264Encoder;
#[cfg(target_os = "macos")]
use shared::platform::macos::network::detect_preferred_interface;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CodecChoice {
    Passthrough,
    H264,
}

#[derive(Debug)]
struct HostConfig {
    bind_address: SocketAddr,
    remote_address: SocketAddr,
    payload_bytes: usize,
    max_payload_bytes: usize,
    frame_interval: Duration,
    codec: CodecChoice,
    width: u32,
    height: u32,
    bitrate: u32,
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

    let mut packetizer = Packetizer::new(
        PacketizerConfig {
            max_payload_bytes: config.max_payload_bytes,
        },
        SequenceNumber::new(0),
    );

    let mut frame_identifier: u32 = 0;
    let mut last_report = Instant::now();
    let mut frames_sent: u64 = 0;

    match config.codec {
        CodecChoice::Passthrough => {
            let mut encoder = PassthroughCodec::default();
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
                send_encoded(
                    &mut sender,
                    &mut packetizer,
                    frame_identifier,
                    timestamp_nanos,
                    &encoded.data,
                )?;

                frames_sent += 1;
                frame_identifier = frame_identifier.wrapping_add(1);
                report(&mut last_report, &mut frames_sent);
                std::thread::sleep(config.frame_interval);
            }
        }
        CodecChoice::H264 => {
            #[cfg(target_os = "macos")]
            {
                let fps = frame_rate_from_interval(config.frame_interval);
                let mut encoder = VideoToolboxH264Encoder::new(
                    config.width,
                    config.height,
                    config.bitrate,
                    fps,
                )?;

                let raw_size = (config.width as usize) * (config.height as usize) * 4;
                loop {
                    let timestamp_nanos = current_time_nanos();
                    let raw_frame = RawFrame {
                        width: config.width,
                        height: config.height,
                        pixel_format: PixelFormat::Rgba8,
                        timestamp: Duration::from_nanos(timestamp_nanos),
                        data: vec![0x7F; raw_size],
                    };

                    let encoded = encoder.encode(&raw_frame)?;
                    send_encoded(
                        &mut sender,
                        &mut packetizer,
                        frame_identifier,
                        timestamp_nanos,
                        &encoded.data,
                    )?;

                    frames_sent += 1;
                    frame_identifier = frame_identifier.wrapping_add(1);
                    report(&mut last_report, &mut frames_sent);
                    std::thread::sleep(config.frame_interval);
                }
            }
            #[cfg(not(target_os = "macos"))]
            {
                return Err("H.264 codec is only supported on macOS".into());
            }
        }
    }
}

fn send_encoded(
    sender: &mut UdpTransport,
    packetizer: &mut Packetizer,
    frame_identifier: u32,
    timestamp_nanos: u64,
    payload: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    let packets = packetizer.packetize(frame_identifier, timestamp_nanos, payload)?;
    for packet in packets {
        let buffer = encode_packet(&packet);
        sender.send(&buffer)?;
    }
    Ok(())
}

fn report(last_report: &mut Instant, frames_sent: &mut u64) {
    if last_report.elapsed() >= Duration::from_secs(1) {
        eprintln!("frames sent: {frames_sent}");
        *last_report = Instant::now();
        *frames_sent = 0;
    }
}

fn current_time_nanos() -> u64 {
    let now = std::time::SystemTime::now();
    let duration = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0));
    duration.as_nanos() as u64
}

fn frame_rate_from_interval(interval: Duration) -> u32 {
    let millis = interval.as_millis().max(1) as u32;
    1000 / millis
}

fn parse_args() -> Result<HostConfig, String> {
    let mut bind_address: Option<SocketAddr> = None;
    let mut remote_address: Option<SocketAddr> = None;
    let mut payload_bytes: usize = 1024;
    let mut max_payload_bytes: usize = 1200;
    let mut frame_interval = Duration::from_millis(16);
    let mut auto_bind_port: Option<u16> = None;
    let mut codec = CodecChoice::Passthrough;
    let mut width: u32 = 320;
    let mut height: u32 = 180;
    let mut bitrate: u32 = 3_000_000;

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
            "--codec" => {
                let value = args.next().ok_or("missing --codec value")?;
                codec = parse_codec(&value)?;
            }
            "--width" => {
                let value = args.next().ok_or("missing --width value")?;
                width = value.parse().map_err(|_| "invalid width")?;
            }
            "--height" => {
                let value = args.next().ok_or("missing --height value")?;
                height = value.parse().map_err(|_| "invalid height")?;
            }
            "--bitrate" => {
                let value = args.next().ok_or("missing --bitrate value")?;
                bitrate = value.parse().map_err(|_| "invalid bitrate")?;
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
        codec,
        width,
        height,
        bitrate,
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
        "usage: host --bind IP:PORT --remote IP:PORT [--payload-bytes N] [--max-payload-bytes N] [--frame-interval-ms N] [--auto-bind-port PORT] [--codec passthrough|h264] [--width N --height N --bitrate N]"
    );
}
