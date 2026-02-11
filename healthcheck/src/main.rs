use shared::core::healthcheck::{HealthcheckKind, HealthcheckPacket};
use shared::transport::udp::UdpTransport;
use shared::transport::{PacketReceiver, PacketSender};
use std::net::SocketAddr;
use std::time::{Duration, Instant};

#[derive(Debug)]
struct HealthcheckConfig {
    bind_address: SocketAddr,
    remote_address: Option<SocketAddr>,
    mode: HealthcheckMode,
    interval: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HealthcheckMode {
    Listen,
    Ping,
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

    if let Err(error) = run_healthcheck(config) {
        eprintln!("healthcheck error: {error}");
        std::process::exit(1);
    }
}

fn run_healthcheck(config: HealthcheckConfig) -> Result<(), Box<dyn std::error::Error>> {
    let mut transport = UdpTransport::bind(config.bind_address)?;
    if let Some(remote) = config.remote_address {
        transport = transport.connect(remote)?;
    }

    match config.mode {
        HealthcheckMode::Listen => run_listener(transport),
        HealthcheckMode::Ping => run_ping(transport, config.interval),
    }
}

fn run_listener(mut transport: UdpTransport) -> Result<(), Box<dyn std::error::Error>> {
    let mut buffer = [0_u8; 64];
    loop {
        let bytes_received = transport.receive(&mut buffer)?;
        let packet = match HealthcheckPacket::decode(&buffer[..bytes_received]) {
            Ok(packet) => packet,
            Err(_) => continue,
        };

        if packet.kind == HealthcheckKind::Ping {
            let response = HealthcheckPacket {
                kind: HealthcheckKind::Pong,
                timestamp_nanos: packet.timestamp_nanos,
            };
            transport.send(&response.encode())?;
        }
    }
}

fn run_ping(
    mut transport: UdpTransport,
    interval: Duration,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut buffer = [0_u8; 64];
    let mut last_send = Instant::now() - interval;

    loop {
        if last_send.elapsed() >= interval {
            let packet = HealthcheckPacket {
                kind: HealthcheckKind::Ping,
                timestamp_nanos: current_time_nanos(),
            };
            transport.send(&packet.encode())?;
            last_send = Instant::now();
        }

        match transport.receive(&mut buffer) {
            Ok(bytes_received) => {
                if let Ok(packet) = HealthcheckPacket::decode(&buffer[..bytes_received]) {
                    if packet.kind == HealthcheckKind::Pong {
                        let now = current_time_nanos();
                        let elapsed_nanos = now.saturating_sub(packet.timestamp_nanos);
                        let elapsed_ms = elapsed_nanos as f64 / 1_000_000.0;
                        eprintln!("pong in {elapsed_ms:.2} ms");
                    }
                }
            }
            Err(_) => {}
        }
    }
}

fn current_time_nanos() -> u64 {
    let now = std::time::SystemTime::now();
    let duration = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0));
    duration.as_nanos() as u64
}

fn parse_args() -> Result<HealthcheckConfig, String> {
    let mut bind_address: Option<SocketAddr> = None;
    let mut remote_address: Option<SocketAddr> = None;
    let mut mode: Option<HealthcheckMode> = None;
    let mut interval = Duration::from_millis(500);

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
            "--listen" => {
                mode = Some(HealthcheckMode::Listen);
            }
            "--ping" => {
                mode = Some(HealthcheckMode::Ping);
            }
            "--interval-ms" => {
                let value = args.next().ok_or("missing --interval-ms value")?;
                let millis: u64 = value.parse().map_err(|_| "invalid interval")?;
                interval = Duration::from_millis(millis);
            }
            "--help" | "-h" => {
                return Err("".to_string());
            }
            _ => return Err(format!("unknown argument: {argument}")),
        }
    }

    let bind_address = bind_address.ok_or("missing --bind")?;
    let mode = mode.ok_or("missing --listen or --ping")?;

    if mode == HealthcheckMode::Ping && remote_address.is_none() {
        return Err("missing --remote for ping".to_string());
    }

    Ok(HealthcheckConfig {
        bind_address,
        remote_address,
        mode,
        interval,
    })
}

fn parse_socket_addr(value: &str) -> Result<SocketAddr, String> {
    value
        .parse()
        .map_err(|_| format!("invalid socket address: {value}"))
}

fn print_usage() {
    eprintln!(
        "usage: healthcheck --bind IP:PORT [--listen | --ping --remote IP:PORT] [--interval-ms N]"
    );
}
