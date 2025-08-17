use std::net::{Ipv4Addr, Ipv6Addr};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandshakeRequest {
    pub protocol: Protocol,
    pub target: TargetAddress,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandshakeResponse {
    pub status: StatusCode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    Tcp = 0,
    Udp = 1,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetAddress {
    pub address: Address,
    pub port: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Address {
    IPv4(Ipv4Addr),
    IPv6(Ipv6Addr),
    Domain(String),
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
    CommandNotSupported = 0x07,
    AddressTypeNotSupported = 0x08,
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
