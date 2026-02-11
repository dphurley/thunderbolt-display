# thunderbolt-display

A minimal, low-latency mac-to-mac extended display prototype with a wired Thunderbolt bridge. The goal is a Duet-style experience with reliability first, then polish.

## Goals
- Extended desktop (OS-level display integration).
- Very low latency and “native-feeling” input/dragging.
- Open source, Rust-first; use small Swift/ObjC shims only where macOS requires it.
- Start with wired Thunderbolt; optional Tailscale path later.

## Non-goals (for v0)
- Cross-platform support.
- Wireless-first.
- Fancy UI or installer.

## Phased plan
1. **Proof of capture + encode** (host)
   - Capture frames from a chosen display.
   - Encode with a low-latency codec (hardware if available).
   - Send over a direct TCP/UDP link on the Thunderbolt bridge.
2. **Proof of decode + present** (client)
   - Decode stream.
   - Present frames in a low-latency full-screen window.
3. **Virtual display integration**
   - Create a virtual display on the host.
   - Route that virtual display’s frames into the encoder.
   - This likely needs a small macOS display/driver component outside pure Rust.
4. **Input backchannel**
   - Forward mouse/keyboard from client to host.
   - Aim for near-native latency.
5. **Hardening**
   - Dropouts, reconnects, versioning, logging, metrics.
6. **Optional Tailscale transport**
   - Add a transport abstraction with a Tailscale-friendly mode.

## Repo layout
- `host/` Rust host app (capture + encode + send)
- `client/` Rust client app (receive + decode + present)

## Current decisions
- Transport: UDP (low latency, wired Thunderbolt link).
- Codec: H.264 or HEVC via hardware encode where available (Apple Silicon).
- Target: macOS current minus one.

## Next steps
- Implement the UDP packetizer and transport core with extensive tests.
- Implement capture + encode (host) and decode + present (client).
- Confirm the exact macOS virtual display API/entitlement path for an extended desktop.

## Local full test (synthetic frames)
1. On the receiving Mac:
   - Run `cargo run -p client -- --bind 0.0.0.0:5000 --remote <HOST_IP>:5001`
2. On the sending Mac:
   - Run `cargo run -p host -- --bind 0.0.0.0:5001 --remote <CLIENT_IP>:5000`

You should see frame/packet counters print once per second on both ends.

## Makefile shortcuts
- `make client CLIENT_REMOTE=<HOST_IP>:5001`
- `make host HOST_REMOTE=<CLIENT_IP>:5000`
- `make client-auto CLIENT_REMOTE=<HOST_IP>:5001` (auto-pick local interface, prefer Thunderbolt Bridge)
- `make host-auto HOST_REMOTE=<CLIENT_IP>:5000` (auto-pick local interface, prefer Thunderbolt Bridge)

## Interface auto-detection
On macOS, `--auto-bind-port` will select the best active IPv4 interface, preferring:\n
1. `bridge0` (common for Thunderbolt Bridge)\n
2. `en*` with a `169.254.x.x` link-local IPv4\n
3. Any active IPv4 interface\n
The selected interface is printed to stderr so you can confirm what was chosen.
