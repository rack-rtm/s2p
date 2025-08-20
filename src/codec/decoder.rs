use crate::codec::types::CodecError::InvalidStatusCode;
use crate::codec::types::{
    CodecError, TcpConnectRequestCodec, TcpConnectResponseCodec, UdpDatagramCodec,
};
use crate::message_types::{
    Host, ConnectStatus, HandshakeRequest, HandshakeResponse, StatusCode, TargetAddress,
    TcpConnectResponse, UdpDatagram,
};
use bytes::{Buf, BytesMut};
use std::net::{Ipv4Addr, Ipv6Addr};
use tokio_util::codec::Decoder;

impl Decoder for TcpConnectRequestCodec {
    type Item = HandshakeRequest;
    type Error = CodecError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.is_empty() {
            return Ok(None);
        }

        let atyp = Self::parse_address_type(src[0])?;

        let required_len = Self::calculate_required_length(src, atyp)?;
        if src.len() < required_len {
            return Ok(None);
        }

        let mut data = src.split_to(required_len);
        data.advance(1);

        let address = Self::parse_address(&mut data, atyp)?;
        let port = data.get_u16();

        Ok(Some(HandshakeRequest {
            target: TargetAddress { host: address, port },
        }))
    }
}

impl Decoder for UdpDatagramCodec {
    type Item = UdpDatagram;
    type Error = CodecError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < 2 {
            return Ok(None);
        }

        let flow_id = src[0];
        let atyp = src[1] & 0b11;

        let required_len = Self::calculate_required_length(src, atyp)?;
        if src.len() < required_len {
            return Ok(None);
        }

        let mut data = src.split_to(required_len);
        data.advance(2); // Skip flow_id and atyp

        let address = Self::parse_address(&mut data, atyp)?;
        let port = data.get_u16();
        let remaining_data = data.to_vec();

        Ok(Some(UdpDatagram {
            flow_id,
            target: TargetAddress { host: address, port },
            data: remaining_data,
        }))
    }
}

impl UdpDatagramCodec {
    fn calculate_required_length(src: &BytesMut, atyp: u8) -> Result<usize, CodecError> {
        match atyp {
            0 => {
                if src.len() < 8 {
                    Ok(usize::MAX) // Not enough data
                } else {
                    Ok(8 + (src.len() - 8)) // flow_id + atyp + IPv4 + port + data
                }
            }
            1 => {
                if src.len() < 20 {
                    Ok(usize::MAX) // Not enough data
                } else {
                    Ok(20 + (src.len() - 20)) // flow_id + atyp + IPv6 + port + data
                }
            }
            2 => {
                if src.len() < 3 {
                    return Ok(usize::MAX); // Not enough data for domain length
                }
                let domain_len = src[2] as usize;
                let header_and_addr_len = 5 + domain_len; // flow_id + atyp + len + domain + port
                if src.len() < header_and_addr_len {
                    Ok(usize::MAX)
                } else {
                    Ok(header_and_addr_len + (src.len() - header_and_addr_len))
                }
            }
            _ => Err(CodecError::InvalidAddressType(atyp)),
        }
    }

    fn parse_address(data: &mut BytesMut, atyp: u8) -> Result<Host, CodecError> {
        match atyp {
            0 => {
                let mut octets = [0u8; 4];
                data.copy_to_slice(&mut octets);
                Ok(Host::IPv4(Ipv4Addr::from(octets)))
            }
            1 => {
                let mut octets = [0u8; 16];
                data.copy_to_slice(&mut octets);
                Ok(Host::IPv6(Ipv6Addr::from(octets)))
            }
            2 => {
                let len = data.get_u8() as usize;
                let mut domain_bytes = vec![0u8; len];
                data.copy_to_slice(&mut domain_bytes);
                let domain = String::from_utf8(domain_bytes)
                    .map_err(|_| CodecError::InvalidDomainEncoding)?;
                Ok(Host::Domain(domain))
            }
            _ => Err(CodecError::InvalidAddressType(atyp)),
        }
    }
}

impl TcpConnectRequestCodec {
    fn parse_address_type(header: u8) -> Result<u8, CodecError> {
        let atyp = header & 0b11;
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

    fn parse_address(data: &mut BytesMut, atyp: u8) -> Result<Host, CodecError> {
        match atyp {
            0 => {
                let mut octets = [0u8; 4];
                data.copy_to_slice(&mut octets);
                Ok(Host::IPv4(Ipv4Addr::from(octets)))
            }
            1 => {
                let mut octets = [0u8; 16];
                data.copy_to_slice(&mut octets);
                Ok(Host::IPv6(Ipv6Addr::from(octets)))
            }
            2 => {
                let len = data.get_u8() as usize;
                let mut domain_bytes = vec![0u8; len];
                data.copy_to_slice(&mut domain_bytes);
                let domain = String::from_utf8(domain_bytes)
                    .map_err(|_| CodecError::InvalidDomainEncoding)?;
                Ok(Host::Domain(domain))
            }
            _ => unreachable!(), // Already validated in parse_address_type
        }
    }
}

impl Decoder for TcpConnectResponseCodec {
    type Item = TcpConnectResponse;
    type Error = CodecError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < 1 {
            return Ok(None);
        }

        let status_byte = src[0];
        let status = ConnectStatus::try_from(status_byte)?;

        src.advance(1);

        Ok(Some(TcpConnectResponse { status }))
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
            0x07 => Ok(StatusCode::AddressTypeNotSupported),
            _ => Err(InvalidStatusCode(value)),
        }
    }
}

impl TryFrom<u8> for ConnectStatus {
    type Error = CodecError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(ConnectStatus::Success),
            0x01 => Ok(ConnectStatus::GeneralFailure),
            0x02 => Ok(ConnectStatus::ConnectionNotAllowed),
            0x03 => Ok(ConnectStatus::NetworkUnreachable),
            0x04 => Ok(ConnectStatus::HostUnreachable),
            0x05 => Ok(ConnectStatus::ConnectionRefused),
            0x06 => Ok(ConnectStatus::TTLExpired),
            0x07 => Ok(ConnectStatus::AddressTypeNotSupported),
            _ => Err(InvalidStatusCode(value)),
        }
    }
}
