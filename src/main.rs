use crate::codec::{HandshakeRequestCodec, HandshakeResponseCodec};
use crate::iroh::{ALPN_S2P_V1, S2pProtocol};
use crate::message_types::{Address, HandshakeRequest, Protocol, TargetAddress};
use ::iroh::Endpoint;
use ::iroh::protocol::Router;
use n0_future::{SinkExt, StreamExt};
use std::net::Ipv4Addr;
use std::thread::sleep;
use std::time::Duration;
use tokio_util::codec::{FramedRead, FramedWrite};

mod codec;
mod iroh;
mod message_types;
mod async_stream;

#[tokio::main]
async fn main() {
    let client_endp = Endpoint::builder().discovery_n0().bind().await.unwrap();
    let server_endp = Endpoint::builder().discovery_n0().bind().await.unwrap();

    let server_node_id = server_endp.node_id().clone();

    let router = Router::builder(server_endp)
        .accept(ALPN_S2P_V1, S2pProtocol)
        .spawn();

    sleep(Duration::from_secs(1));

    let connection = client_endp
        .connect(server_node_id, ALPN_S2P_V1.as_ref())
        .await
        .unwrap();

    let (writer, reader) = connection.open_bi().await.unwrap();
    let mut framed_writer = FramedWrite::new(writer, HandshakeRequestCodec);
    framed_writer
        .send(HandshakeRequest {
            protocol: Protocol::Tcp,
            target: TargetAddress {
                address: Address::IPv4(Ipv4Addr::new(127, 0, 0, 1)),
                port: 1234,
            },
        })
        .await
        .unwrap();

    let mut framed_reader = FramedRead::new(reader, HandshakeResponseCodec);
    let option = framed_reader.next().await;
    println!("{:?}", option);

    let mut writer = framed_writer.into_inner();

    writer.write(b"hello").await.unwrap();
    sleep(Duration::from_secs(1));
    let result = writer.write(b"world").await;
    println!("{:?}", result);

    sleep(Duration::from_secs(5));
    let _ = router.shutdown();
}
