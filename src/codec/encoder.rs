use crate::codec::types::{
    CodecError, TcpConnectRequestCodec, TcpConnectResponseCodec, UdpDatagramCodec,
};
use crate::message_types::{Host, TcpConnectRequest, TcpConnectResponse, UdpDatagram};
use bytes::{BufMut, BytesMut};
use tokio_util::codec::Encoder;
impl Encoder<UdpDatagram> for UdpDatagramCodec {
    type Error = CodecError;

    fn encode(&mut self, datagram: UdpDatagram, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let SerializedAddress {
            atyp,
            domain_length,
            address,
        } = SerializedAddress::from_address(&datagram.target.host)?;

        dst.put_u8(datagram.flow_id);
        dst.put_u8(atyp);

        if let Some(domain_length) = domain_length {
            dst.put_u8(domain_length);
        }

        dst.put_slice(address.as_slice());
        dst.put_u16(datagram.target.port);
        dst.put_slice(&datagram.data);

        Ok(())
    }
}

impl Encoder<TcpConnectRequest> for TcpConnectRequestCodec {
    type Error = CodecError;

    fn encode(&mut self, req: TcpConnectRequest, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let SerializedAddress {
            atyp,
            domain_length,
            address,
        } = SerializedAddress::from_address(&req.target.host)?;

        dst.put_u8(atyp);

        if let Some(domain_length) = domain_length {
            dst.put_u8(domain_length);
        }

        dst.put_slice(address.as_slice());
        dst.put_u16(req.target.port);

        Ok(())
    }
}

impl Encoder<TcpConnectResponse> for TcpConnectResponseCodec {
    type Error = CodecError;

    fn encode(&mut self, resp: TcpConnectResponse, dst: &mut BytesMut) -> Result<(), Self::Error> {
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

impl SerializedAddress {
    fn from_address(address: &Host) -> Result<SerializedAddress, CodecError> {
        let (atyp, domain_length, address) = match address {
            Host::IPv4(ip) => (0u8, None, ip.octets().to_vec()),
            Host::IPv6(ip) => (1u8, None, ip.octets().to_vec()),
            Host::Domain(domain) => {
                if domain.len() > 255 {
                    return Err(CodecError::DomainTooLong(domain.len()));
                }
                (2u8, Some(domain.len() as u8), domain.as_bytes().to_vec())
            }
        };

        Ok(SerializedAddress::new(atyp, domain_length, address))
    }
}
