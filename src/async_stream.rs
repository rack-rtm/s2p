use iroh::endpoint::{RecvStream, SendStream};
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf, Result};
use tokio::net::{TcpStream, UdpSocket};

#[derive(Debug)]
pub enum AsyncStream {
    Tcp(TcpStream),
    Udp(UdpSocket),
    Iroh(RecvStream, SendStream),
}

impl AsyncStream {
    pub fn from_tcp(stream: TcpStream) -> Self {
        Self::Tcp(stream)
    }

    pub fn from_udp(socket: UdpSocket) -> Self {
        Self::Udp(socket)
    }

    pub fn from_iroh(reader: RecvStream, writer: SendStream) -> Self {
        Self::Iroh(reader, writer)
    }

    pub fn is_tcp(&self) -> bool {
        matches!(self, Self::Tcp(_))
    }

    pub fn is_udp(&self) -> bool {
        matches!(self, Self::Udp(_))
    }

    pub fn is_iroh(&self) -> bool {
        matches!(self, Self::Iroh(_, _))
    }
}

impl AsyncRead for AsyncStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<Result<()>> {
        match self.get_mut() {
            AsyncStream::Tcp(stream) => Pin::new(stream).poll_read(cx, buf),
            AsyncStream::Udp(socket) => {
                let mut temp_buf = vec![0u8; buf.remaining()];
                match socket.try_recv(&mut temp_buf) {
                    Ok(n) => {
                        buf.put_slice(&temp_buf[..n]);
                        Poll::Ready(Ok(()))
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        let mut read_buf = ReadBuf::new(&mut temp_buf);
                        Pin::new(socket).poll_recv(cx, &mut read_buf).map(|res| {
                            res.map(|_| {
                                let filled = read_buf.filled();
                                buf.put_slice(filled);
                            })
                        })
                    }
                    Err(e) => Poll::Ready(Err(e)),
                }
            }
            AsyncStream::Iroh(recv_stream, _) => Pin::new(recv_stream).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for AsyncStream {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize>> {
        match self.get_mut() {
            AsyncStream::Tcp(stream) => Pin::new(stream).poll_write(cx, buf),
            AsyncStream::Udp(socket) => match socket.try_send(buf) {
                Ok(n) => Poll::Ready(Ok(n)),
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    Pin::new(socket).poll_send(cx, buf)
                }
                Err(e) => Poll::Ready(Err(e)),
            },
            AsyncStream::Iroh(_, send_stream) => Pin::new(send_stream)
                .poll_write(cx, buf)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e)),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        match self.get_mut() {
            AsyncStream::Tcp(stream) => Pin::new(stream).poll_flush(cx),
            AsyncStream::Udp(_) => {
                // UDP doesn't need flushing
                Poll::Ready(Ok(()))
            }
            AsyncStream::Iroh(_, send_stream) => Pin::new(send_stream)
                .poll_flush(cx)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e)),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        match self.get_mut() {
            AsyncStream::Tcp(stream) => Pin::new(stream).poll_shutdown(cx),
            AsyncStream::Udp(_) => Poll::Ready(Ok(())),
            AsyncStream::Iroh(_, send_stream) => Pin::new(send_stream)
                .poll_shutdown(cx)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e)),
        }
    }
}
