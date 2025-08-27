use crate::codec::{CodecError, TcpConnectRequestCodec, TcpConnectResponseCodec};
use crate::iroh_stream::IrohStream;
use crate::message_types::{ConnectStatusCode, Host, TcpConnectRequest, TcpConnectResponse};
use iroh::endpoint::{RecvStream, SendStream};
use n0_future::SinkExt;
use std::io;
use std::io::ErrorKind;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::io::copy_bidirectional;
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_stream::StreamExt;
use tokio_util::codec::{FramedRead, FramedWrite};
use tracing::{error, info};

#[derive(Clone)]
pub struct TcpProxyTimeouts {
    pub connection_timeout: Duration,
    pub dns_resolution_timeout: Duration,
    pub handshake_timeout: Duration,
}

impl Default for TcpProxyTimeouts {
    fn default() -> Self {
        Self {
            connection_timeout: Duration::from_secs(10),
            dns_resolution_timeout: Duration::from_secs(5),
            handshake_timeout: Duration::from_secs(30),
        }
    }
}

pub struct TcpProxyHandlerHandler {
    timeouts: TcpProxyTimeouts,
}

impl TcpProxyHandlerHandler {

    pub fn new() -> Self {
        Self {
            timeouts: TcpProxyTimeouts::default(),
        }
    }

    pub fn with_timeouts(timeouts: TcpProxyTimeouts) -> Self {
        Self { timeouts }
    }
    pub async fn handle_stream(&self, writer: SendStream, reader: RecvStream) {
        let mut framed_writer = FramedWrite::new(writer, TcpConnectResponseCodec);
        let mut framed_reader = FramedRead::new(reader, TcpConnectRequestCodec);

        let handshake_request = match self.read_handshake_request(&mut framed_reader).await {
            Ok(request) => request,
            Err(StreamError::IoError(error)) => {
                error!("Stream IO error during handshake: {:?}", error);
                return;
            }
            Err(StreamError::ProtocolError(status_code)) => {
                error!("Protocol error during handshake: {:?}", status_code);
                let response = TcpConnectResponse {
                    status: status_code,
                };
                if let Err(e) = framed_writer.send(response).await {
                    error!("Failed to send error response: {:?}", e);
                }
                return;
            }
        };

        let mut target_stream =
            match self.establish_connection_to_target(handshake_request.clone()).await {
                Ok(stream) => stream,
                Err(StreamError::IoError(error)) => {
                    error!("Stream IO error establishing connection: {:?}", error);
                    return;
                }
                Err(StreamError::ProtocolError(status_code)) => {
                    error!("Protocol error establishing connection: {:?}", status_code);
                    let response = TcpConnectResponse::new(status_code);
                    if let Err(e) = framed_writer.send(response).await {
                        error!("Failed to send error response: {:?}", e);
                    }
                    return;
                }
            };
        let _ = framed_writer.send(TcpConnectResponse::success()).await;

        let mut iroh_stream =
            IrohStream::new(framed_reader.into_inner(), framed_writer.into_inner());
        info!("Starting bi directional stream copy");
        match copy_bidirectional(&mut iroh_stream, &mut target_stream).await {
            Ok((r, s)) => {
                info!("Stream copy completed: {} bytes read, {} bytes written", r, s);
            }
            Err(error) => {
                error!("Stream IO error during copy: {:?}", error);
            }
        }
    }

    async fn establish_connection_to_target(
        &self,
        handshake_request: TcpConnectRequest,
    ) -> Result<TcpStream, StreamError> {
        let target_address = handshake_request.target;

        let socket_addr = self.resolve_address(&target_address.host, target_address.port).await?;

        let tcp_stream = timeout(self.timeouts.connection_timeout, TcpStream::connect(socket_addr))
            .await
            .map_err(|_| StreamError::ProtocolError(ConnectStatusCode::TTLExpired))?
            .map_err(|e| match e.kind() {
                ErrorKind::ConnectionRefused => {
                    StreamError::ProtocolError(ConnectStatusCode::ConnectionRefused)
                }
                ErrorKind::TimedOut => StreamError::ProtocolError(ConnectStatusCode::TTLExpired),
                ErrorKind::NotFound | ErrorKind::AddrNotAvailable => {
                    StreamError::ProtocolError(ConnectStatusCode::HostUnreachable)
                }
                _ => StreamError::ProtocolError(ConnectStatusCode::GeneralFailure),
            })?;

        Ok(tcp_stream)
    }

    async fn resolve_address(&self, address: &Host, port: u16) -> Result<SocketAddr, StreamError> {
        match address {
            Host::IPv4(ip) => {
                info!("Using IPv4 address: {}:{}", ip, port);
                Ok(SocketAddr::from((*ip, port)))
            }
            Host::IPv6(ip) => {
                info!("Using IPv6 address: [{}]:{}", ip, port);
                Ok(SocketAddr::from((*ip, port)))
            }
            Host::Domain(domain) => {
                info!("Resolving domain: {}:{}", domain, port);
                let addr = format!("{}:{}", domain, port);
                match timeout(self.timeouts.dns_resolution_timeout, tokio::net::lookup_host(&addr)).await {
                    Ok(Ok(mut addrs)) => match addrs.next() {
                        Some(resolved) => {
                            info!("Domain {} resolved to {}", domain, resolved);
                            Ok(resolved)
                        }
                        None => {
                            error!("DNS resolution for {} returned no results", domain);
                            Err(StreamError::ProtocolError(
                                ConnectStatusCode::HostUnreachable,
                            ))
                        }
                    },
                    Ok(Err(e)) => {
                        error!("DNS resolution failed for {}: {}", domain, e);
                        Err(StreamError::ProtocolError(
                            ConnectStatusCode::HostUnreachable,
                        ))
                    }
                    Err(_) => {
                        error!("DNS resolution for {} timed out", domain);
                        Err(StreamError::ProtocolError(
                            ConnectStatusCode::HostUnreachable,
                        ))
                    }
                }
            }
        }
    }

    async fn read_handshake_request(
        &self,
        framed_reader: &mut FramedRead<RecvStream, TcpConnectRequestCodec>,
    ) -> Result<TcpConnectRequest, StreamError> {
        match timeout(self.timeouts.handshake_timeout, framed_reader.next()).await {
            Ok(Some(Ok(handshake_request))) => {
                info!("Successfully read handshake request");
                Ok(handshake_request)
            }
            Ok(Some(Err(CodecError::Io(error)))) => {
                error!("IO error reading handshake: {}", error);
                Err(StreamError::IoError(error))
            }
            Ok(Some(Err(codec_error))) => {
                error!("Codec error reading handshake: {:?}", codec_error);
                Err(StreamError::ProtocolError(ConnectStatusCode::from(
                    codec_error,
                )))
            }
            Ok(None) => {
                error!("Stream ended during handshake");
                Err(StreamError::IoError(io::Error::new(
                    ErrorKind::UnexpectedEof,
                    "stream ended during handshake",
                )))
            }
            Err(_) => {
                error!("Handshake request timed out");
                Err(StreamError::IoError(io::Error::new(
                    ErrorKind::TimedOut,
                    "handshake request timed out",
                )))
            }
        }
    }
}

#[derive(Debug)]
enum StreamError {
    IoError(io::Error),
    ProtocolError(ConnectStatusCode),
}

impl From<CodecError> for ConnectStatusCode {
    fn from(value: CodecError) -> Self {
        match value {
            CodecError::DomainTooLong(_) => ConnectStatusCode::HostUnreachable,
            CodecError::InvalidDomainEncoding => ConnectStatusCode::HostUnreachable,
            CodecError::InvalidAddressType(_) => ConnectStatusCode::AddressTypeNotSupported,
            _ => ConnectStatusCode::GeneralFailure,
        }
    }
}
