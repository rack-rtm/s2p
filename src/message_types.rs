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
    pub status: ConnectStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ConnectStatus {
    Success = 0x00,
    GeneralFailure = 0x01,
    ConnectionNotAllowed = 0x02,
    NetworkUnreachable = 0x03,
    HostUnreachable = 0x04,
    ConnectionRefused = 0x05,
    TTLExpired = 0x06,
    AddressTypeNotSupported = 0x07,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandshakeResponse {
    pub status: StatusCode,
}

impl HandshakeResponse {
    pub fn new(status: StatusCode) -> Self {
        Self { status }
    }

    pub fn success() -> Self {
        Self {
            status: StatusCode::Success,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum StatusCode {
    Success = 0x00,
    GeneralFailure = 0x01,
    ConnectionNotAllowed = 0x02,
    NetworkUnreachable = 0x03,
    HostUnreachable = 0x04,
    ConnectionRefused = 0x05,
    TTLExpired = 0x06,
    AddressTypeNotSupported = 0x07,
}
