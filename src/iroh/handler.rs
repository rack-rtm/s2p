use crate::iroh::tcp_handler::TcpProxyHandlerHandler;
use crate::iroh::types::S2pProtocol;
use bytes::Bytes;
use iroh::endpoint::Connection;
use iroh::protocol::{AcceptError, ProtocolHandler};
use n0_future::StreamExt;

impl ProtocolHandler for S2pProtocol {
    fn accept(
        &self,
        connection: Connection,
    ) -> impl Future<Output = Result<(), AcceptError>> + Send {
        Box::pin(async move {
            let connection_clone = connection.clone();

            let bi_stream_task = tokio::spawn(async move {
                while let Ok((writer, reader)) = connection.accept_bi().await {
                    tokio::spawn(async move {
                        TcpProxyHandlerHandler {}
                            .handle_stream(writer, reader)
                            .await;
                    });
                }
            });

            let datagram_task = tokio::spawn(async move {
                let result = connection_clone.read_datagram().await;
                while let _ = connection_clone.read_datagram().await {}
            });

            // Wait for either task to complete (they run concurrently)
            tokio::select! {
                _ = bi_stream_task => {},
                _ = datagram_task => {},
            }

            Ok(())
        })
    }
}

impl S2pProtocol {
    async fn handle_datagram(datagram: Bytes) {
        // TODO: Implement datagram protocol handling
        // This could be UDP proxy functionality similar to SOCKS5 UDP
        println!("Received datagram of {} bytes", datagram.len());
    }
}
