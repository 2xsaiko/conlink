use std::io;
use std::net::SocketAddr;

use futures::task::Context;
use tokio::io::AsyncWriteExt;
use tokio::macros::support::{Pin, Poll};
use tokio::net::TcpStream;
use tokio::prelude::AsyncRead;

pub struct NetClient {
    lines: TcpStream,
    addr: SocketAddr,
}

impl NetClient {
    pub async fn new(
        stream: TcpStream,
    ) -> NetClient {
        let addr = stream.peer_addr().expect("could not get peer address");
        NetClient {
            lines: stream,
            addr,
        }
    }

    pub fn get_addr(&self) -> SocketAddr {
        self.addr
    }

    pub async fn send_line(&mut self, line: &[u8]) -> io::Result<()> {
        self.lines.write_all(line).await
    }
}

impl AsyncRead for NetClient {
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.lines).poll_read(cx, buf)
    }
}