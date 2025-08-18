use crate::codec::{CodecError, HandshakeRequestCodec, HandshakeResponseCodec};
use crate::iroh::handler::StreamError::ProtocolError;
use crate::iroh::types::S2pProtocol;
use crate::iroh_stream::IrohStream;
use crate::message_types::{Address, HandshakeRequest, HandshakeResponse, StatusCode};
use iroh::endpoint::{Connection, RecvStream, SendStream};
use iroh::protocol::{AcceptError, ProtocolHandler};
use n0_future::boxed::BoxFuture;
use n0_future::{SinkExt, StreamExt};
use std::io;
use std::io::ErrorKind;
use std::net::SocketAddr;
use tokio::io::copy_bidirectional;
use tokio::net::TcpStream;
use tokio::time::{Duration, timeout};
use tokio_util::codec::{FramedRead, FramedWrite};
use tracing::{error, info};

impl ProtocolHandler for S2pProtocol {
    fn accept(&self, connection: Connection) -> BoxFuture<Result<(), AcceptError>> {
        let self_clone = self.clone();

        Box::pin(async move {
            while let Ok((writer, reader)) = connection.accept_bi().await {
                let self_clone = self_clone.clone();
                info!("New bidirectional stream from");

                tokio::spawn(async move {
                    let _ = self_clone.handle_stream(writer, reader).await;
                });
            }

            Ok(())
        })
    }
}

impl S2pProtocol {
    async fn handle_stream(&self, writer: SendStream, reader: RecvStream) {
        let mut framed_writer = FramedWrite::new(writer, HandshakeResponseCodec);
        let mut framed_reader = FramedRead::new(reader, HandshakeRequestCodec);

        let handshake_request = match Self::read_handshake_request(&mut framed_reader).await {
            Ok(request) => request,
            Err(StreamError::IoError(error)) => {
                error!("Stream IO error during handshake: {:?}", error);
                return;
            }
            Err(ProtocolError(status_code)) => {
                error!("Protocol error during handshake: {:?}", status_code);
                let response = HandshakeResponse {
                    status: status_code,
                };
                if let Err(e) = framed_writer.send(response).await {
                    error!("Failed to send error response: {:?}", e);
                }
                return;
            }
        };

        let mut target_stream =
            match Self::establish_connection_to_target(handshake_request.clone()).await {
                Ok(stream) => stream,
                Err(StreamError::IoError(error)) => {
                    error!("Stream IO error establishing connection: {:?}", error);
                    return;
                }
                Err(ProtocolError(status_code)) => {
                    error!("Protocol error establishing connection: {:?}", status_code);
                    let response = HandshakeResponse::new(status_code);
                    if let Err(e) = framed_writer.send(response).await {
                        error!("Failed to send error response: {:?}", e);
                    }
                    return;
                }
            };
        let _ = framed_writer.send(HandshakeResponse::success()).await;

        let mut iroh_stream =
            IrohStream::new(framed_reader.into_inner(), framed_writer.into_inner());
        info!("Starting bi directional stream copy");
        if let Err(error) = copy_bidirectional(&mut iroh_stream, &mut target_stream).await {
            error!("Stream IO error during copy: {:?}", error);
        }
    }

    async fn establish_connection_to_target(
        handshake_request: HandshakeRequest,
    ) -> Result<TcpStream, StreamError> {
        let target_address = handshake_request.target;

        let socket_addr =
            Self::resolve_address(&target_address.address, target_address.port).await?;

        let tcp_stream = timeout(Duration::from_secs(10), TcpStream::connect(socket_addr))
            .await
            .map_err(|_| ProtocolError(StatusCode::TTLExpired))?
            .map_err(|e| match e.kind() {
                ErrorKind::ConnectionRefused => ProtocolError(StatusCode::ConnectionRefused),
                ErrorKind::TimedOut => ProtocolError(StatusCode::TTLExpired),
                ErrorKind::NotFound | ErrorKind::AddrNotAvailable => {
                    ProtocolError(StatusCode::HostUnreachable)
                }
                _ => ProtocolError(StatusCode::GeneralFailure),
            })?;

        Ok(tcp_stream)
    }

    async fn resolve_address(address: &Address, port: u16) -> Result<SocketAddr, StreamError> {
        match address {
            Address::IPv4(ip) => {
                info!("Using IPv4 address: {}:{}", ip, port);
                Ok(SocketAddr::from((*ip, port)))
            }
            Address::IPv6(ip) => {
                info!("Using IPv6 address: [{}]:{}", ip, port);
                Ok(SocketAddr::from((*ip, port)))
            }
            Address::Domain(domain) => {
                info!("Resolving domain: {}:{}", domain, port);
                let addr = format!("{}:{}", domain, port);
                match timeout(Duration::from_secs(5), tokio::net::lookup_host(&addr)).await {
                    Ok(Ok(mut addrs)) => match addrs.next() {
                        Some(resolved) => {
                            info!("Domain {} resolved to {}", domain, resolved);
                            Ok(resolved)
                        }
                        None => {
                            error!("DNS resolution for {} returned no results", domain);
                            Err(ProtocolError(StatusCode::HostUnreachable))
                        }
                    },
                    Ok(Err(e)) => {
                        error!("DNS resolution failed for {}: {}", domain, e);
                        Err(ProtocolError(StatusCode::HostUnreachable))
                    }
                    Err(_) => {
                        error!("DNS resolution for {} timed out", domain);
                        Err(ProtocolError(StatusCode::HostUnreachable))
                    }
                }
            }
        }
    }

    async fn read_handshake_request(
        framed_reader: &mut FramedRead<RecvStream, HandshakeRequestCodec>,
    ) -> Result<HandshakeRequest, StreamError> {
        match timeout(Duration::from_secs(30), framed_reader.next()).await {
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
                Err(ProtocolError(StatusCode::from(codec_error)))
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
    ProtocolError(StatusCode),
}

impl From<CodecError> for StatusCode {
    fn from(value: CodecError) -> Self {
        match value {
            CodecError::DomainTooLong(_) => StatusCode::HostUnreachable,
            CodecError::InvalidDomainEncoding => StatusCode::HostUnreachable,
            CodecError::InvalidAddressType(_) => StatusCode::AddressTypeNotSupported,
            _ => unreachable!(),
        }
    }
}
