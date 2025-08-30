use crate::iroh::tcp_handler::TcpProxyHandlerHandler;
use crate::iroh::types::S2pProtocol;
use crate::iroh::udp_handler::UdpProxyHandlerHandler;
use iroh::endpoint::Connection;
use iroh::protocol::{AcceptError, ProtocolHandler};

impl ProtocolHandler for S2pProtocol {
    fn accept(
        &self,
        connection: Connection,
    ) -> impl Future<Output = Result<(), AcceptError>> + Send {
        Box::pin(async move {
            let connection_clone = connection.clone();
            let handler_clone = self.clone();
            let socket_factory_clone = self.socket_factory.clone();
            
            let bi_stream_task = tokio::spawn(async move {
                while let Ok((writer, reader)) = connection.accept_bi().await {
                    let handler_clone = handler_clone.clone();
                    tokio::spawn(async move {
                        TcpProxyHandlerHandler::with_timeouts_and_socket_factory(
                            handler_clone.proxy_timeouts,
                            handler_clone.socket_factory.clone(),
                        )
                        .handle_stream(writer, reader)
                        .await;
                    });
                }
            });

            let datagram_task = tokio::spawn(async move {
                let udp_handler = UdpProxyHandlerHandler::with_socket_factory(
                    socket_factory_clone
                );
                while let Ok(datagram) = connection_clone.read_datagram().await {
                    udp_handler
                        .handle_datagram(&connection_clone, datagram)
                        .await;
                }
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
