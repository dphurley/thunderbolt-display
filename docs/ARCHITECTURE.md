# Architecture

This project is structured as two Rust binaries plus a small macOS-native display component if required for virtual displays.

## Host
- Capture frames from a virtual display.
- Encode with a low-latency pipeline.
- Transport over a direct Thunderbolt bridge link.

## Client
- Receive frames.
- Decode to GPU-friendly surfaces.
- Present with vsync-aware timing and minimal buffering.

## Transport
- Initial target: direct wired link with a fixed IP (Thunderbolt bridge) using UDP.
- Future: Tailscale-backed transport for remote use.

## Codec
- H.264 or HEVC via hardware encode/decode when available on Apple Silicon.

## Open questions
- The best macOS-native API to create a virtual display and feed frames into capture.
- The minimal entitlement/signing path for a developer-friendly install.
- The lowest-latency codec and transport combo on current macOS hardware.
