use crate::async_stream::AsyncStream;
use crate::codec::{CodecError, HandshakeRequestCodec, HandshakeResponseCodec};
use crate::iroh::handler::StreamError::ProtocolError;
use crate::iroh::types::S2pProtocol;
use crate::message_types::{Address, HandshakeRequest, HandshakeResponse, Protocol, StatusCode};
use iroh::endpoint::{Connection, RecvStream, SendStream};
use iroh::protocol::{AcceptError, ProtocolHandler};
use n0_future::boxed::BoxFuture;
use n0_future::{SinkExt, StreamExt};
use std::io;
use std::io::ErrorKind;
use std::net::SocketAddr;
use tokio::io::copy_bidirectional;
use tokio::net::{TcpStream, UdpSocket};
use tokio_util::codec::{FramedRead, FramedWrite};

impl ProtocolHandler for S2pProtocol {
    fn accept(&self, connection: Connection) -> BoxFuture<Result<(), AcceptError>> {
        let self_clone = self.clone();
        Box::pin(async move {
            while let Ok((writer, reader)) = connection.accept_bi().await {
                let self_clone = self_clone.clone();
                let _ = tokio::spawn(async move {
                    let _ = self_clone.handle_stream(writer, reader).await;
                });
            }

            Ok(())
        })
    }
}

impl S2pProtocol {
    async fn handle_stream(&self, writer: SendStream, reader: RecvStream) {
        let mut framed_reader = FramedRead::new(reader, HandshakeRequestCodec);
        let mut framed_writer = FramedWrite::new(writer, HandshakeResponseCodec);

        let handshake_decode_result = Self::read_handshake_request(&mut framed_reader).await;
        let mut async_stream = match handshake_decode_result {
            Ok(handshake_request) => {
                let stream = Self::establish_connection_to_target(handshake_request)
                    .await
                    .unwrap();
                framed_writer
                    .send(HandshakeResponse::success())
                    .await
                    .unwrap();
                stream
            }
            Err(ProtocolError(status)) => {
                framed_writer
                    .send(HandshakeResponse::new(status))
                    .await
                    .unwrap();
                return;
            }
            Err(StreamError::IoError(io_error)) => {
                return;
            }
        };

        let _ = copy_bidirectional(
            &mut AsyncStream::from_iroh(framed_reader.into_inner(), framed_writer.into_inner()),
            &mut async_stream,
        )
        .await;
    }

    async fn establish_connection_to_target(
        handshake_request: HandshakeRequest,
    ) -> Result<AsyncStream, StreamError> {
        let target_address = handshake_request.target;
        let protocol = handshake_request.protocol;

        let socket_addr =
            match Self::resolve_address(&target_address.address, target_address.port).await {
                Ok(addr) => addr,
                Err(_) => return Err(ProtocolError(StatusCode::HostUnreachable)),
            };

        let async_stream = match protocol {
            Protocol::Tcp => AsyncStream::Tcp(TcpStream::connect(socket_addr).await.unwrap()),
            Protocol::Udp => {
                let udp_socket = UdpSocket::bind("0.0.0.0:0").await.unwrap();
                udp_socket.connect(socket_addr).await.unwrap();
                AsyncStream::Udp(udp_socket)
            }
        };

        Ok(async_stream)
    }

    async fn resolve_address(address: &Address, port: u16) -> Result<SocketAddr, ()> {
        match address {
            Address::IPv4(ip) => Ok(SocketAddr::from((*ip, port))),
            Address::IPv6(ip) => Ok(SocketAddr::from((*ip, port))),
            Address::Domain(domain) => {
                let addr = format!("{}:{}", domain, port);
                match tokio::net::lookup_host(&addr).await {
                    Ok(mut addrs) => match addrs.next() {
                        Some(resolved) => Ok(resolved),
                        None => todo!(), //  Err(S2pError::DnsNoResults)
                    },
                    Err(e) => todo!(), // Err(S2pError::DnsResolutionFailed(e))
                }
            }
        }
    }

    async fn read_handshake_request(
        framed_reader: &mut FramedRead<RecvStream, HandshakeRequestCodec>,
    ) -> Result<HandshakeRequest, StreamError> {
        match framed_reader.next().await {
            Some(Ok(handshake_request)) => Ok(handshake_request),
            Some(Err(CodecError::Io(error))) => Err(StreamError::IoError(error)),
            Some(Err(codec_error)) => Err(ProtocolError(StatusCode::from(codec_error))),
            None => Err(StreamError::IoError(io::Error::new(
                ErrorKind::UnexpectedEof,
                "stream ended during handshake",
            ))),
        }
    }
}

#[derive(Debug)]
enum StreamError {
    IoError(std::io::Error),
    ProtocolError(StatusCode),
}

impl From<CodecError> for StatusCode {
    fn from(value: CodecError) -> Self {
        match value {
            CodecError::DomainTooLong(_) => StatusCode::HostUnreachable,
            CodecError::InvalidDomainEncoding => StatusCode::HostUnreachable,
            CodecError::InvalidAddressType(_) => StatusCode::AddressTypeNotSupported,
            CodecError::InvalidStatusCode(_) => StatusCode::GeneralFailure,
            _ => unreachable!(),
        }
    }
}
