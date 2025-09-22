use std::future::Future;
use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use tokio::net::{TcpStream, UdpSocket};

pub trait SocketFactory: Send + Sync + std::fmt::Debug {
    fn create_tcp_connection(
        &self,
        addr: SocketAddr,
    ) -> Pin<Box<dyn Future<Output = Result<TcpStream, io::Error>> + Send + '_>>;

    fn create_udp_socket(
        &self,
        bind_addr: &str,
    ) -> Pin<Box<dyn Future<Output = Result<UdpSocket, io::Error>> + Send + '_>>;
}

#[derive(Debug, Clone)]
pub struct DefaultSocketFactory;

impl DefaultSocketFactory {
    pub fn new() -> Self {
        Self
    }

    pub fn arc() -> Arc<dyn SocketFactory> {
        Arc::new(Self::new())
    }
}

impl SocketFactory for DefaultSocketFactory {
    fn create_tcp_connection(
        &self,
        addr: SocketAddr,
    ) -> Pin<Box<dyn Future<Output = Result<TcpStream, io::Error>> + Send + '_>> {
        Box::pin(async move { TcpStream::connect(addr).await })
    }

    fn create_udp_socket(
        &self,
        bind_addr: &str,
    ) -> Pin<Box<dyn Future<Output = Result<UdpSocket, io::Error>> + Send + '_>> {
        let bind_addr = bind_addr.to_string();
        Box::pin(async move { UdpSocket::bind(bind_addr).await })
    }
}

impl Default for DefaultSocketFactory {
    fn default() -> Self {
        Self::new()
    }
}
