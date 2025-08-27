use iroh::protocol::Router;
use std::time::Duration;
use iroh::SecretKey;
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

async fn run_s2p_server() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("Starting S2P proxy server");

    // Create Iroh endpoint for server with discovery
    let bytes = hex::decode("fb6b6c4c3af8412f4b9a87fb74977f8b3e34e6fdf752525d318895a5a17efa80")?;

    let endpoint = iroh::endpoint::Endpoint::builder()
        .discovery_n0()
        .secret_key(SecretKey::from_bytes(bytes.as_slice().try_into()?))
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