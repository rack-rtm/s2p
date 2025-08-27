use iroh::protocol::Router;
use iroh::RelayMode::Custom;
use iroh::{RelayMap, RelayNode, SecretKey};
use std::net::{Ipv4Addr, SocketAddrV4};
use std::time::Duration;
use iroh::endpoint::{TransportConfig, VarInt};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tracing::{error, info};

use s2p::{S2pProtocol, ALPN_S2P_V1};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    info!("Starting S2P Server with Echo Service");

    // Start both services concurrently
    tokio::select! {
        result = run_s2p_server() => {
            if let Err(e) = result {
                error!("S2P Server error: {}", e);
            }
        }
        result = run_echo_server() => {
            if let Err(e) = result {
                error!("Echo Server error: {}", e);
            }
        }
    }

    Ok(())
}

fn create_300mbps_config() -> TransportConfig {
    // 300 Mbps = 37,500,000 bytes/s
    const TARGET_BANDWIDTH: u32 = 37_500_000; // 300 Mbps in bytes/s
    const EXPECTED_RTT: u32 = 100; // ms

    // Calculate stream window: bandwidth * RTT
    // 37,500,000 bytes/s * 0.1s = 3,750,000 bytes
    const STREAM_WINDOW: u32 = TARGET_BANDWIDTH / 1000 * EXPECTED_RTT;

    let mut config = TransportConfig::default();

    config
        // Set stream receive window for 300 Mbps throughput
        .stream_receive_window(VarInt::from_u32(STREAM_WINDOW))
        // Connection-level window (larger than stream window)
        .receive_window(VarInt::from_u32(STREAM_WINDOW * 2))
        // Send window should be generous for high throughput
        .send_window((STREAM_WINDOW * 8) as u64)
        // Adjust buffer sizes for high bandwidth
        .datagram_receive_buffer_size(Some(STREAM_WINDOW as usize))
        .datagram_send_buffer_size(STREAM_WINDOW as usize);

    config
}

async fn run_s2p_server() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("Starting S2P proxy server");

    // Create Iroh endpoint for server with discovery
    let bytes = hex::decode("fb6b6c4c3af8412f4b9a87fb74977f8b3e34e6fdf752525d318895a5a17efa80")?;
    let endpoint = iroh::endpoint::Endpoint::builder()
        .discovery_n0()
        .secret_key(SecretKey::from_bytes(bytes.as_slice().try_into()?))
        .transport_config(create_300mbps_config())
        .bind_addr_v4(SocketAddrV4::new(Ipv4Addr::new(0, 0 ,0 ,0), 5555))
        .bind()
        .await?;

    // let endpoint = iroh::endpoint::Endpoint::builder()
    //     .discovery_n0()
    //     .bind()
    //     .await?;

    info!("Server Node ID: {}", endpoint.node_id());
    let public_key = endpoint.node_id();

    // Get the secret key (private key)
    let secret_key = endpoint.secret_key();

    // Print the keys
    println!("=== Iroh Endpoint Keys ===");
    println!("Public Key (NodeId): {}", public_key);
    let secret_bytes = secret_key.to_bytes();
    println!("Secret key bytes (hex): {}", hex::encode(secret_bytes));

    let node_id = endpoint.node_id();
    // info!("S2P proxy server starting...");
    // info!("Server Node ID: {}", node_id);

    // Use Router to handle the S2P protocol
    let _router = Router::builder(endpoint)
        .accept(ALPN_S2P_V1, S2pProtocol)
        .spawn();

    // Keep server running indefinitely
    loop {
        tokio::time::sleep(Duration::from_secs(60)).await;
    }
}

async fn run_echo_server() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("Starting TCP echo server on localhost:1235");
    
    let listener = TcpListener::bind("127.0.0.1:1235").await?;
    info!("Echo server listening on 127.0.0.1:1235");

    while let Ok((mut stream, addr)) = listener.accept().await {
        info!("Echo server: New connection from {}", addr);
        
        tokio::spawn(async move {
            let mut buffer = [0; 1024];
            loop {
                match stream.read(&mut buffer).await {
                    Ok(0) => {
                        info!("Echo server: Client {} disconnected", addr);
                        break;
                    }
                    Ok(n) => {
                        let received = String::from_utf8_lossy(&buffer[..n]);
                        info!("Echo server: Received from {}: '{}'", addr, received.trim());
                        
                        // Echo back the same message
                        if let Err(e) = stream.write_all(&buffer[..n]).await {
                            error!("Echo server: Failed to write response to {}: {}", addr, e);
                            break;
                        }
                    }
                    Err(e) => {
                        error!("Echo server: Read error from {}: {}", addr, e);
                        break;
                    }
                }
            }
        });
    }

    Ok(())
}