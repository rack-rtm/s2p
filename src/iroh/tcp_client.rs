use crate::codec::{CodecError, TcpConnectRequestCodec, TcpConnectResponseCodec};
use crate::iroh_stream::IrohStream;
use crate::message_types::{
    ConnectStatusCode, TargetAddress, TcpConnectRequest, TcpConnectResponse,
};
use iroh::endpoint::{Connection, RecvStream, SendStream};
use n0_future::SinkExt;
use std::io;
use std::time::Duration;
use tokio::io::copy_bidirectional;
use tokio::net::{TcpListener, TcpStream};
use tokio::time::timeout;
use tokio_stream::StreamExt;
use tokio_util::codec::{FramedRead, FramedWrite};
use tracing::{error, info};

#[derive(Clone)]
pub struct TcpClientTimeouts {
    pub request_timeout: Duration,
    pub response_timeout: Duration,
}

impl Default for TcpClientTimeouts {
    fn default() -> Self {
        Self {
            request_timeout: Duration::from_secs(10),
            response_timeout: Duration::from_secs(30),
        }
    }
}

pub struct TcpClient {
    connection: Connection,
    timeouts: TcpClientTimeouts,
}

impl TcpClient {
    pub fn new(connection: Connection) -> Self {
        Self {
            connection,
            timeouts: TcpClientTimeouts::default(),
        }
    }

    pub fn with_timeouts(connection: Connection, timeouts: TcpClientTimeouts) -> Self {
        Self {
            connection,
            timeouts,
        }
    }

    pub async fn connect(&self, target: TargetAddress) -> Result<IrohStream, TcpClientError> {
        let (writer, reader) = self
            .connection
            .open_bi()
            .await
            .map_err(|e| TcpClientError::IoError(io::Error::new(io::ErrorKind::Other, e)))?;

        let mut framed_writer = FramedWrite::new(writer, TcpConnectRequestCodec);
        let mut framed_reader = FramedRead::new(reader, TcpConnectResponseCodec);

        let connect_request = TcpConnectRequest { target };

        timeout(self.timeouts.request_timeout, framed_writer.send(connect_request))
            .await
            .map_err(|_| TcpClientError::IoError(io::Error::new(io::ErrorKind::TimedOut, "request timeout")))?
            .map_err(|e| TcpClientError::IoError(io::Error::new(io::ErrorKind::Other, e.to_string())))?;

        let response = self.read_connect_response(&mut framed_reader).await?;

        if response.status != ConnectStatusCode::Success {
            return Err(TcpClientError::ProtocolError(response.status));
        }

        info!("Successfully established connection to target");

        Ok(IrohStream::new(
            framed_reader.into_inner(),
            framed_writer.into_inner(),
        ))
    }

    async fn read_connect_response(
        &self,
        framed_reader: &mut FramedRead<RecvStream, TcpConnectResponseCodec>,
    ) -> Result<TcpConnectResponse, TcpClientError> {
        match timeout(self.timeouts.response_timeout, framed_reader.next()).await {
            Ok(Some(Ok(response))) => {
                info!("Received connect response: {:?}", response.status);
                Ok(response)
            }
            Ok(Some(Err(e))) => {
                error!("Codec error reading response: {:?}", e);
                Err(TcpClientError::InvalidRequest)
            }
            Ok(None) => {
                error!("Stream ended during response");
                Err(TcpClientError::IoError(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "stream ended during response",
                )))
            }
            Err(_) => {
                error!("Response timed out");
                Err(TcpClientError::IoError(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "response timed out",
                )))
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TcpClientError {
    #[error("IO error: {0}")]
    IoError(#[from] io::Error),

    #[error("Protocol error: {0:?}")]
    ProtocolError(ConnectStatusCode),

    #[error("Invalid request")]
    InvalidRequest,
}
