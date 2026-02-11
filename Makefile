HOST_BIND ?= 0.0.0.0:5001
HOST_REMOTE ?= 192.168.0.2:5000
CLIENT_BIND ?= 0.0.0.0:5000
CLIENT_REMOTE ?= 192.168.0.1:5001
PAYLOAD_BYTES ?= 65536
MAX_PAYLOAD_BYTES ?= 1200
FRAME_INTERVAL_MS ?= 16
MAX_PACKET_BYTES ?= 2048
MAX_IN_FLIGHT_FRAMES ?= 8

.PHONY: test host client help

help:
	@echo "Targets:"
	@echo "  make test"
	@echo "  make host HOST_BIND=0.0.0.0:5001 HOST_REMOTE=<client_ip>:5000"
	@echo "  make client CLIENT_BIND=0.0.0.0:5000 CLIENT_REMOTE=<host_ip>:5001"

test:
	cargo test

host:
	cargo run -p host -- \
		--bind $(HOST_BIND) \
		--remote $(HOST_REMOTE) \
		--payload-bytes $(PAYLOAD_BYTES) \
		--max-payload-bytes $(MAX_PAYLOAD_BYTES) \
		--frame-interval-ms $(FRAME_INTERVAL_MS)

client:
	cargo run -p client -- \
		--bind $(CLIENT_BIND) \
		--remote $(CLIENT_REMOTE) \
		--max-packet-bytes $(MAX_PACKET_BYTES) \
		--max-in-flight-frames $(MAX_IN_FLIGHT_FRAMES)
