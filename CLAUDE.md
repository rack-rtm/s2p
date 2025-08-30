# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Common Development Commands

### Building and Testing
- `cargo build` - Build the project
- `cargo test` - Run all tests 
- `cargo check` - Check code without building
- `cargo run --example tcp_connect_example` - Run the main example demonstrating S2P functionality

### Running Examples
- Main example: `RUST_LOG=info cargo run --example tcp_connect_example` - Shows complete S2P proxy workflow with logging

## Project Architecture

This is an S2P (Simple-Proxy-Proticik) implementation built on top of Iroh's networking stack. The project provides a proxy protocol that allows TCP connections to be established through an Iroh node network.

### Core Components

1. **Message Types** (`src/message_types.rs`): Defines the core protocol data structures:
   - `TcpConnectRequest/Response` - Protocol handshake messages
   - `UdpDatagram` - UDP packet encapsulation 
   - `TargetAddress` - Address specification (IPv4/IPv6/Domain)
   - `ConnectStatusCode` - Connection result codes

2. **Codec Layer** (`src/codec/`): Binary encoding/decoding for protocol messages:
   - `TcpConnectRequestCodec/TcpConnectResponseCodec` - TCP handshake codecs
   - `UdpDatagramCodec` - UDP message codec
   - Built on tokio-util's codec framework

3. **Iroh Integration** (`src/iroh/`): 
   - `S2pProtocol` - Main protocol handler implementing Iroh's protocol trait
   - `TcpClient` - Client-side interface for establishing proxied connections
   - `TcpHandler/UdpHandler` - Server-side connection handlers
   - Uses ALPN identifier "s2p/1"

4. **Stream Abstraction** (`src/iroh_stream.rs`): Provides async stream interface over Iroh connections

### Key Constants
- `ALPN_S2P_V1 = "s2p/1"` - Protocol identifier for Iroh ALPN negotiation

### Usage Pattern
1. Server creates Iroh endpoint and registers S2P protocol with Router
2. Client connects to server using node ID and ALPN_S2P_V1 
3. Client creates TcpClient with the connection
4. Client calls `connect(target_address)` to establish proxied TCP connection
5. Resulting stream can be used for normal TCP I/O operations

### Testing
- Integration tests in `tests/integration_test.rs`
- Example in `examples/tcp_connect_example.rs` demonstrates full workflow including test target server