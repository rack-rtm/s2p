use crate::codec::types::CodecError::InvalidStatusCode;
use crate::codec::types::{CodecError, HandshakeRequestCodec, HandshakeResponseCodec};
use crate::message_types::{
    Address, HandshakeRequest, HandshakeResponse, Protocol, StatusCode, TargetAddress,
};
use bytes::{Buf, BytesMut};
use std::net::{Ipv4Addr, Ipv6Addr};
use tokio_util::codec::Decoder;

impl Decoder for HandshakeRequestCodec {
    type Item = HandshakeRequest;
    type Error = CodecError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.is_empty() {
            return Ok(None);
        }

        let header = src[0];
        let protocol = Self::parse_protocol(header)?;
        let atyp = Self::parse_address_type(header)?;

        let required_len = Self::calculate_required_length(src, atyp)?;
        if src.len() < required_len {
            return Ok(None);
        }

        let mut data = src.split_to(required_len);
        data.advance(1);

        let address = Self::parse_address(&mut data, atyp)?;
        let port = data.get_u16();

        Ok(Some(HandshakeRequest {
            protocol,
            target: TargetAddress { address, port },
        }))
    }
}

impl Decoder for HandshakeResponseCodec {
    type Item = HandshakeResponse;
    type Error = CodecError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < 1 {
            return Ok(None);
        }

        let status_byte = src[0];
        let status = StatusCode::try_from(status_byte)?;

        src.advance(1);

        Ok(Some(HandshakeResponse { status }))
    }
}

impl HandshakeRequestCodec {
    fn parse_protocol(header: u8) -> Result<Protocol, CodecError> {
        match (header >> 7) & 0x01 {
            0 => Ok(Protocol::Tcp),
            1 => Ok(Protocol::Udp),
            _ => unreachable!(),
        }
    }

    fn parse_address_type(header: u8) -> Result<u8, CodecError> {
        let atyp = (header >> 5) & 0x03;
        match atyp {
            0..=2 => Ok(atyp),
            _ => Err(CodecError::InvalidAddressType(atyp)),
        }
    }

    fn calculate_required_length(src: &BytesMut, atyp: u8) -> Result<usize, CodecError> {
        match atyp {
            0 => Ok(7),  // header + IPv4 + port
            1 => Ok(19), // header + IPv6 + port
            2 => {
                if src.len() < 2 {
                    return Ok(usize::MAX); // Force "not enough data"
                }
                let domain_len = src[1] as usize;
                Ok(4 + domain_len) // header + length + domain + port
            }
            _ => unreachable!(),
        }
    }

    fn parse_address(data: &mut BytesMut, atyp: u8) -> Result<Address, CodecError> {
        match atyp {
            0 => {
                let mut octets = [0u8; 4];
                data.copy_to_slice(&mut octets);
                Ok(Address::IPv4(Ipv4Addr::from(octets)))
            }
            1 => {
                let mut octets = [0u8; 16];
                data.copy_to_slice(&mut octets);
                Ok(Address::IPv6(Ipv6Addr::from(octets)))
            }
            2 => {
                let len = data.get_u8() as usize;
                let mut domain_bytes = vec![0u8; len];
                data.copy_to_slice(&mut domain_bytes);
                let domain = String::from_utf8(domain_bytes)
                    .map_err(|_| CodecError::InvalidDomainEncoding)?;
                Ok(Address::Domain(domain))
            }
            _ => unreachable!(), // Already validated in parse_address_type
        }
    }
}

impl TryFrom<u8> for StatusCode {
    type Error = CodecError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(StatusCode::Success),
            0x01 => Ok(StatusCode::GeneralFailure),
            0x02 => Ok(StatusCode::ConnectionNotAllowed),
            0x03 => Ok(StatusCode::NetworkUnreachable),
            0x04 => Ok(StatusCode::HostUnreachable),
            0x05 => Ok(StatusCode::ConnectionRefused),
            0x06 => Ok(StatusCode::TTLExpired),
            0x07 => Ok(StatusCode::CommandNotSupported),
            0x08 => Ok(StatusCode::AddressTypeNotSupported),
            _ => Err(InvalidStatusCode(value)),
        }
    }
}
