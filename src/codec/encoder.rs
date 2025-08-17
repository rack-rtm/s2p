use crate::codec::types::{CodecError, HandshakeRequestCodec, HandshakeResponseCodec};
use crate::message_types::{Address, HandshakeRequest, HandshakeResponse, Protocol};
use bytes::{BufMut, BytesMut};
use tokio_util::codec::Encoder;

impl Encoder<HandshakeRequest> for HandshakeRequestCodec {
    type Error = CodecError;

    fn encode(&mut self, req: HandshakeRequest, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let protocol_bit = match req.protocol {
            Protocol::Tcp => 0u8,
            Protocol::Udp => 1u8,
        };

        let SerializedAddress {
            atyp,
            domain_length,
            address,
        } = Self::serialize_address(&req.target.address)?;

        let first_byte = (protocol_bit << 7) | (atyp << 5);
        dst.put_u8(first_byte);

        if let Some(domain_length) = domain_length {
            dst.put_u8(domain_length);
        }

        dst.put_slice(address.as_slice());
        dst.put_u16(req.target.port);

        Ok(())
    }
}

impl Encoder<HandshakeResponse> for HandshakeResponseCodec {
    type Error = CodecError;

    fn encode(&mut self, resp: HandshakeResponse, dst: &mut BytesMut) -> Result<(), Self::Error> {
        dst.put_u8(resp.status as u8);
        Ok(())
    }
}

struct SerializedAddress {
    atyp: u8,
    domain_length: Option<u8>,
    address: Vec<u8>,
}

impl SerializedAddress {
    pub fn new(atyp: u8, domain_length: Option<u8>, address: Vec<u8>) -> Self {
        Self {
            atyp,
            domain_length,
            address,
        }
    }
}

impl HandshakeRequestCodec {
    fn serialize_address(address: &Address) -> Result<SerializedAddress, CodecError> {
        let (at, domain_length, address) = match address {
            Address::IPv4(ip) => (0u8, None, ip.octets().to_vec()),
            Address::IPv6(ip) => (1u8, None, ip.octets().to_vec()),
            Address::Domain(domain) => {
                if domain.len() > 255 {
                    return Err(CodecError::DomainTooLong(domain.len()));
                }
                (2u8, None, domain.as_bytes().to_vec())
            }
        };

        Ok(SerializedAddress::new(at, domain_length, address))
    }
}
