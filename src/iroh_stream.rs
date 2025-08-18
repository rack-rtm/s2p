use iroh::endpoint::{RecvStream, SendStream};
use pin_project::pin_project;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf, Result};

#[derive(Debug)]
#[pin_project]
pub struct IrohStream {
    #[pin]
    pub recv: RecvStream,
    #[pin]
    pub send: SendStream,
}

impl IrohStream {
    pub fn new(recv: RecvStream, send: SendStream) -> Self {
        Self { recv, send }
    }
}
impl AsyncRead for IrohStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<Result<()>> {
        AsyncRead::poll_read(self.project().recv, cx, buf)
    }
}

impl AsyncWrite for IrohStream {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize>> {
        AsyncWrite::poll_write(self.project().send, cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        AsyncWrite::poll_flush(self.project().send, cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        AsyncWrite::poll_shutdown(self.project().send, cx)
    }
}
