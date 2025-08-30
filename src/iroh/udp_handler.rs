use crate::codec::{CodecError, UdpDatagramCodec};
use crate::iroh::socket_factory::SocketFactory;
use crate::message_types::{ConnectStatusCode, Host, UdpDatagram};
use bytes::{Bytes, BytesMut};
use iroh::endpoint::Connection;
use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::sync::Mutex;
use tokio::time::timeout;
use tokio_util::codec::{Decoder, Encoder};
use tracing::{error, info};

pub struct UdpProxyHandlerHandler {
    flows: Arc<Mutex<HashMap<u8, Arc<UdpSocket>>>>,
    socket_factory: Arc<dyn SocketFactory>,
}

impl UdpProxyHandlerHandler {
    
    pub fn with_socket_factory(socket_factory: Arc<dyn SocketFactory>) -> Self {
        Self {
            flows: Arc::new(Mutex::new(HashMap::new())),
            socket_factory,
        }
    }

    pub async fn handle_datagram(&self, connection: &Connection, datagram: Bytes) {
        match self.process_datagram(connection, datagram).await {
            Ok(_) => info!("Successfully processed UDP datagram"),
            Err(e) => error!("Failed to process UDP datagram: {:?}", e),
        }
    }

    async fn process_datagram(
        &self,
        connection: &Connection,
        datagram: Bytes,
    ) -> Result<(), UdpError> {
        let udp_datagram = self.parse_udp_datagram(datagram)?;
        let flow_id = udp_datagram.flow_id;

        let target_clone = udp_datagram.target.clone();
        let socket = {
            let mut flows = self.flows.lock().await;
            if let Some(existing_socket) = flows.get(&flow_id) {
                existing_socket.clone()
            } else {
                let new_socket = Arc::new(
                    self.socket_factory
                        .create_udp_socket("0.0.0.0:0")
                        .await
                        .map_err(UdpError::IoError)?,
                );

                let socket_clone = new_socket.clone();
                let connection_clone = connection.clone();
                let flows_clone = self.flows.clone();

                tokio::spawn(async move {
                    Self::listen_for_responses(
                        flow_id,
                        socket_clone,
                        connection_clone,
                        flows_clone,
                        target_clone,
                    )
                    .await;
                });

                flows.insert(flow_id, new_socket.clone());
                new_socket
            }
        };

        let target_address = &udp_datagram.target;
        let socket_addr = Self::resolve_address(&target_address.host, target_address.port).await?;

        socket
            .send_to(&udp_datagram.data, socket_addr)
            .await
            .map_err(UdpError::IoError)?;

        info!(
            "Sent {} bytes to target for flow_id {}",
            udp_datagram.data.len(),
            flow_id
        );
        Ok(())
    }

    fn parse_udp_datagram(&self, datagram: Bytes) -> Result<UdpDatagram, UdpError> {
        let mut codec = UdpDatagramCodec;
        let mut buf = BytesMut::from(datagram.as_ref());

        match codec.decode(&mut buf) {
            Ok(Some(udp_datagram)) => Ok(udp_datagram),
            Ok(None) => Err(UdpError::ProtocolError(ConnectStatusCode::GeneralFailure)),
            Err(e) => Err(UdpError::CodecError(e)),
        }
    }

    fn encode_udp_datagram(&self, datagram: UdpDatagram) -> Result<Bytes, UdpError> {
        let mut codec = UdpDatagramCodec;
        let mut buf = BytesMut::new();

        codec
            .encode(datagram, &mut buf)
            .map_err(UdpError::CodecError)?;

        Ok(buf.freeze())
    }

    async fn listen_for_responses(
        flow_id: u8,
        socket: Arc<UdpSocket>,
        connection: Connection,
        flows: Arc<Mutex<HashMap<u8, Arc<UdpSocket>>>>,
        target: crate::message_types::TargetAddress,
    ) {
        let mut buffer = [0u8; 65536];

        loop {
            match timeout(Duration::from_secs(60), socket.recv_from(&mut buffer)).await {
                Ok(Ok((len, _from_addr))) => {
                    let response_data = buffer[..len].to_vec();
                    let response_datagram = UdpDatagram {
                        flow_id,
                        target: target.clone(),
                        data: response_data,
                    };

                    if let Ok(encoded_response) =
                        Self::encode_udp_datagram_static(response_datagram)
                    {
                        if let Err(e) = connection.send_datagram(encoded_response) {
                            error!(
                                "Failed to send UDP response for flow_id {}: {:?}",
                                flow_id, e
                            );
                            break;
                        } else {
                            info!("Sent {} bytes back to client for flow_id {}", len, flow_id);
                        }
                    } else {
                        error!("Failed to encode UDP response for flow_id {}", flow_id);
                        break;
                    }
                }
                Ok(Err(e)) => {
                    error!("Socket error for flow_id {}: {:?}", flow_id, e);
                    break;
                }
                Err(_) => {
                    info!("UDP socket timeout for flow_id {}, cleaning up", flow_id);
                    break;
                }
            }
        }

        let mut flows_guard = flows.lock().await;
        flows_guard.remove(&flow_id);
        info!("Removed flow_id {} from active flows", flow_id);
    }

    fn encode_udp_datagram_static(datagram: UdpDatagram) -> Result<Bytes, UdpError> {
        let mut codec = UdpDatagramCodec;
        let mut buf = BytesMut::new();

        codec
            .encode(datagram, &mut buf)
            .map_err(UdpError::CodecError)?;

        Ok(buf.freeze())
    }

    async fn resolve_address(address: &Host, port: u16) -> Result<SocketAddr, UdpError> {
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
                match timeout(Duration::from_secs(5), tokio::net::lookup_host(&addr)).await {
                    Ok(Ok(mut addrs)) => match addrs.next() {
                        Some(resolved) => {
                            info!("Domain {} resolved to {}", domain, resolved);
                            Ok(resolved)
                        }
                        None => {
                            error!("DNS resolution for {} returned no results", domain);
                            Err(UdpError::ProtocolError(ConnectStatusCode::HostUnreachable))
                        }
                    },
                    Ok(Err(e)) => {
                        error!("DNS resolution failed for {}: {}", domain, e);
                        Err(UdpError::ProtocolError(ConnectStatusCode::HostUnreachable))
                    }
                    Err(_) => {
                        error!("DNS resolution for {} timed out", domain);
                        Err(UdpError::ProtocolError(ConnectStatusCode::HostUnreachable))
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
enum UdpError {
    IoError(io::Error),
    ProtocolError(ConnectStatusCode),
    CodecError(CodecError),
}
