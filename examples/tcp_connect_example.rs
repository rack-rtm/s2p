use iroh::protocol::Router;
use std::net::Ipv4Addr;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tracing::{error, info};

use s2p::{Host, S2pProtocol, TargetAddress, TcpClient, ALPN_S2P_V1};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    // Create a channel to share the server node ID with the client
    let (server_id_tx, server_id_rx) = oneshot::channel();

    // Start test target server first
    let target_server_handle = tokio::spawn(run_target_server());

    // Give target server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    tokio::select! {
        result = run_s2p_server(server_id_tx) => {
            if let Err(e) = result {
                error!("S2P Server error: {}", e);
            }
        }
        result = run_client_example(server_id_rx) => {
            if let Err(e) = result {
                error!("Client error: {}", e);
            }
        }
        result = target_server_handle => {
            if let Err(e) = result {
                error!("Target server error: {}", e);
            }
        }
    }

    Ok(())
}

// Simple target server that echoes received data
async fn run_target_server() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("Starting target server on localhost:1234");
    let listener = TcpListener::bind("127.0.0.1:1234").await?;
    info!("Target server listening on 127.0.0.1:1234");

    while let Ok((mut stream, addr)) = listener.accept().await {
        info!("Target server: New connection from {}", addr);
        tokio::spawn(async move {
            let mut buffer = [0; 1024];
            loop {
                match stream.read(&mut buffer).await {
                    Ok(0) => {
                        info!("Target server: Client disconnected");
                        break;
                    }
                    Ok(n) => {
                        let received = String::from_utf8_lossy(&buffer[..n]);
                        info!("Target server: Received: '{}'", received.trim());

                        // Echo back with a prefix
                        let response = format!("Echo: {}", received);
                        if let Err(e) = stream.write_all(response.as_bytes()).await {
                            error!("Target server: Failed to write response: {}", e);
                            break;
                        }
                    }
                    Err(e) => {
                        error!("Target server: Read error: {}", e);
                        break;
                    }
                }
            }
        });
    }

    Ok(())
}

async fn run_s2p_server(
    server_id_tx: oneshot::Sender<iroh::NodeId>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("Starting S2P proxy server");

    // Create Iroh endpoint for server with discovery
    let endpoint = iroh::endpoint::Endpoint::builder()
        .discovery_n0()
        .bind()
        .await?;

    let node_id = endpoint.node_id();
    info!("S2P proxy server starting...");
    info!("Server Node ID: {}", node_id);

    // Send the node ID to the client
    if let Err(_) = server_id_tx.send(node_id) {
        error!("Failed to send server node ID to client");
    }

    // Use Router to handle the S2P protocol
    let _router = Router::builder(endpoint)
        .accept(ALPN_S2P_V1, S2pProtocol::new())
        .spawn();

    // Keep server running
    tokio::time::sleep(Duration::from_secs(30)).await;
    info!("S2P server shutting down");

    Ok(())
}

async fn run_client_example(
    server_id_rx: oneshot::Receiver<iroh::NodeId>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("Starting S2P client example");

    // Create Iroh endpoint for client with discovery
    let client_endpoint = iroh::endpoint::Endpoint::builder()
        .discovery_n0()
        .bind()
        .await?;

    info!("Client Node ID: {}", client_endpoint.node_id());

    // Wait for the server node ID (this happens when server is ready)
    let server_node_id = server_id_rx
        .await
        .map_err(|_| "Failed to receive server node ID")?;
    info!("Received server node ID: {}", server_node_id);

    // Give the server router a moment to be fully ready
    tokio::time::sleep(Duration::from_millis(500)).await;

    info!("Attempting to demonstrate S2P client functionality...");

    // Now we can demonstrate the actual S2P usage pattern
    demonstrate_s2p_usage_pattern(&client_endpoint, server_node_id).await?;

    Ok(())
}

async fn demonstrate_s2p_usage_pattern(
    client_endpoint: &iroh::endpoint::Endpoint,
    server_node_id: iroh::NodeId,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("=== S2P Usage Pattern ===");
    info!(
        "1. Connecting to S2P server with node ID: {}",
        server_node_id
    );

    // Connect to the S2P server using the node ID
    let connection = client_endpoint
        .connect(server_node_id, ALPN_S2P_V1.as_bytes())
        .await?;
    info!("✓ Connected to S2P server");

    // Create TcpClient with the S2P connection
    let client = TcpClient::new(connection);
    info!("✓ Created TcpClient");

    // Connect to localhost:1234 through the S2P proxy
    let target = TargetAddress {
        host: Host::IPv4(Ipv4Addr::new(127, 0, 0, 1)),
        port: 1234,
    };

    info!("2. Connecting to localhost:1234 through S2P proxy...");
    match client.connect(target).await {
        Ok(mut stream) => {
            info!("✓ Connected to localhost:1234 via S2P proxy");

            // Send test data
            let message = "Hello from S2P client!\n";
            stream.write_all(message.as_bytes()).await?;
            info!("✓ Sent: '{}'", message.trim());

            // Read response
            let mut buffer = [0; 1024];
            let n = stream.read(&mut buffer).await?;
            let response = String::from_utf8_lossy(&buffer[..n]);
            info!("✓ Received: '{}'", response.trim());

            info!("=== S2P Usage Pattern Complete ===");
        }
        Err(e) => {
            error!("✗ Failed to connect via S2P proxy: {}", e);
            return Err(Box::new(e));
        }
    }

    Ok(())
}
