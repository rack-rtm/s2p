use std::net::{Ipv4Addr, Ipv6Addr};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UdpDatagram {
    pub flow_id: u8,
    pub target: TargetAddress,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TcpConnectRequest {
    pub target: TargetAddress,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TcpConnectResponse {
    pub status: ConnectStatusCode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetAddress {
    pub host: Host,
    pub port: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Host {
    IPv4(Ipv4Addr),
    IPv6(Ipv6Addr),
    Domain(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandshakeRequest {
    pub target: TargetAddress,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ConnectStatusCode {
    Success = 0x00,
    GeneralFailure = 0x01,
    ConnectionNotAllowed = 0x02,
    NetworkUnreachable = 0x03,
    HostUnreachable = 0x04,
    ConnectionRefused = 0x05,
    TTLExpired = 0x06,
    AddressTypeNotSupported = 0x07,
}

impl TcpConnectResponse {
    pub fn new(status: ConnectStatusCode) -> Self {
        Self { status }
    }

    pub fn success() -> Self {
        Self {
            status: ConnectStatusCode::Success,
        }
    }
}
