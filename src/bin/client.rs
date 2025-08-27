use std::io::{self, Write};
use std::net::Ipv4Addr;
use iroh::SecretKey;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{error, info, warn};

use s2p::{TcpClient, TargetAddress, Host, ALPN_S2P_V1};

struct ClientState {
    connection: Option<iroh::endpoint::Connection>,
    stream: Option<s2p::iroh_stream::IrohStream>,
    endpoint: iroh::endpoint::Endpoint,
}

impl ClientState {
    async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let bytes = hex::decode("fb6b6c4c3af8412f4b9a87fb74977f8b3e34e6fdf752525d318895a5a17efa80")?;

        let endpoint = iroh::endpoint::Endpoint::builder()
            .discovery_n0()
            .secret_key(SecretKey::from_bytes(bytes.as_slice().try_into()?))
            .bind()
            .await?;

        info!("Client Node ID: {}", endpoint.node_id());
        let public_key = endpoint.node_id();

        // Get the secret key (private key)
        let secret_key = endpoint.secret_key();

        // Print the keys
        println!("=== Iroh Endpoint Keys ===");
        println!("Public Key (NodeId): {}", public_key);
        let secret_bytes = secret_key.to_bytes();
        println!("Secret key bytes (hex): {}", hex::encode(secret_bytes));
        Ok(Self {
            connection: None,
            stream: None,
            endpoint,
        })
    }

    async fn create_new_connection(&mut self, server_node_id: iroh::NodeId) -> Result<(), Box<dyn std::error::Error>> {
        info!("Creating new connection to server: {}", server_node_id);
        
        // Close existing stream if any
        if self.stream.is_some() {
            info!("Closing existing stream");
            self.stream = None;
        }

        // Close existing connection if any
        if let Some(conn) = &self.connection {
            info!("Closing existing connection");
            conn.close(0u32.into(), b"new connection requested");
        }

        // Create new connection
        let connection = self.endpoint.connect(server_node_id, ALPN_S2P_V1.as_bytes()).await?;
        info!("✓ Connected to S2P server");
        
        self.connection = Some(connection);
        Ok(())
    }

    async fn create_new_stream(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let connection = self.connection.as_ref()
            .ok_or("No active connection. Use 'c:' to create a new connection first.")?;

        info!("Creating new stream");
        
        // Close existing stream if any
        if self.stream.is_some() {
            info!("Closing existing stream");
            self.stream = None;
        }

        // Create TcpClient and connect through S2P proxy
        let client = TcpClient::new(connection.clone());
        
        let target = TargetAddress {
            host: Host::IPv4(Ipv4Addr::new(127, 0, 0, 1)),
            port: 1235,
        };

        let stream = client.connect(target).await?;
        info!("✓ Connected to echo server via S2P proxy");
        
        self.stream = Some(stream);
        Ok(())
    }

    async fn send_message(&mut self, message: &str) -> Result<String, Box<dyn std::error::Error>> {
        let stream = self.stream.as_mut()
            .ok_or("No active stream. Use 's:' to create a new stream first.")?;

        // Send message with newline for proper protocol handling
        let message_with_newline = format!("{}\n", message);
        stream.write_all(message_with_newline.as_bytes()).await?;
        info!("Sent: '{}'", message);

        // Read response
        let mut buffer = [0; 1024];
        let n = stream.read(&mut buffer).await?;
        let response = String::from_utf8_lossy(&buffer[..n]).trim().to_string();
        
        Ok(response)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("S2P Interactive Client");
    println!("Commands:");
    println!("  c:<node_id>  - Create new connection to server");
    println!("  s:           - Create new stream (requires active connection)");
    println!("  <message>    - Send message to echo server");
    println!("  quit         - Exit client");
    println!();

    let mut client_state = ClientState::new().await?;

    loop {
        print!("> ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.is_empty() {
            continue;
        }

        if input == "quit" {
            println!("Goodbye!");
            break;
        }

        if let Some(node_id_str) = input.strip_prefix("c:") {
            let node_id_str = node_id_str.trim();
            if node_id_str.is_empty() {
                println!("Error: Please provide server node ID after 'c:'");
                continue;
            }

            match node_id_str.parse::<iroh::NodeId>() {
                Ok(node_id) => {
                    match client_state.create_new_connection(node_id).await {
                        Ok(()) => println!("✓ New connection created"),
                        Err(e) => println!("✗ Failed to create connection: {}", e),
                    }
                }
                Err(e) => {
                    println!("✗ Invalid node ID format: {}", e);
                }
            }
        } else if input.starts_with("s:") {
            match client_state.create_new_stream().await {
                Ok(()) => println!("✓ New stream created"),
                Err(e) => println!("✗ Failed to create stream: {}", e),
            }
        } else {
            // Regular message
            match client_state.send_message(input).await {
                Ok(response) => {
                    println!("Echo: {}", response);
                }
                Err(e) => {
                    println!("✗ Failed to send message: {}", e);
                }
            }
        }
    }

    Ok(())
}