use ::iroh::endpoint::TransportConfig;
use ::iroh::protocol::Router;
use ::iroh::Endpoint;
use n0_future::{SinkExt, StreamExt};
use s2p::codec::{TcpConnectRequestCodec, TcpConnectResponseCodec};
use s2p::iroh::{S2pProtocol, ALPN_S2P_V1};
use s2p::message_types::{HandshakeRequest, Host, TargetAddress};
use std::net::Ipv4Addr;
use std::thread::sleep;
use std::time::Duration;
use tokio_util::codec::{FramedRead, FramedWrite};
use s2p::TcpConnectRequest;

#[tokio::test]
async fn test_s2p_protocol() {
    env_logger::init();
    let client_endp = Endpoint::builder()
        .transport_config(TransportConfig::default())
        .discovery_n0()
        .bind()
        .await
        .unwrap();
    let server_endp = Endpoint::builder().discovery_n0().bind().await.unwrap();

    let server_node_id = server_endp.node_id().clone();

    let router = Router::builder(server_endp)
        .accept(ALPN_S2P_V1, S2pProtocol)
        .spawn();

    sleep(Duration::from_secs(10));

    let connection = client_endp
        .connect(server_node_id, ALPN_S2P_V1.as_ref())
        .await
        .unwrap();

    let (writer, reader) = connection.open_bi().await.unwrap();
    let mut framed_writer = FramedWrite::new(writer, TcpConnectRequestCodec);
    framed_writer
        .send(TcpConnectRequest {
            target: TargetAddress {
                host: Host::IPv4(Ipv4Addr::new(127, 0, 0, 1)),
                port: 1234,
            },
        })
        .await
        .unwrap();

    let mut framed_reader = FramedRead::new(reader, TcpConnectResponseCodec);
    let option = framed_reader.next().await;
    println!("Got response kek {:?}", option);

    let mut writer = framed_writer.into_inner();

    println!("Writing hello");
    writer.write(b"hello").await.unwrap();
    println!("Wrote hello");
    sleep(Duration::from_secs(1));
    let result = writer.write(b"world").await;
    println!("{:?}", result);

    sleep(Duration::from_secs(5));
    let _ = router.shutdown();
}