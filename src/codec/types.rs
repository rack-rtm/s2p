#[derive(Debug, Default)]
pub struct UdpDatagramCodec;

#[derive(Debug, Default)]
pub struct TcpConnectRequestCodec;

#[derive(Debug, Default)]
pub struct TcpConnectResponseCodec;

#[derive(Debug, thiserror::Error)]
pub enum CodecError {
    #[error("IO error")]
    Io(#[from] std::io::Error),

    #[error("Domain name too long: {0} bytes (max 255)")]
    DomainTooLong(usize),

    #[error("Invalid domain name encoding")]
    InvalidDomainEncoding,

    #[error("Invalid address type: {0}")]
    InvalidAddressType(u8),

    #[error("Invalid status code: {0}")]
    InvalidStatusCode(u8),
}
