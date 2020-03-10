use std::net::SocketAddr;

use futures::SinkExt;
use futures::task::Context;
use tokio::macros::support::{Pin, Poll};
use tokio::net::TcpStream;
use tokio::stream::Stream;
use tokio_util::codec::{Framed, LinesCodec, LinesCodecError};

pub struct NetClient {
    lines: Framed<TcpStream, LinesCodec>,
    addr: SocketAddr,
}

impl NetClient {
    pub async fn new(
        stream: TcpStream,
    ) -> NetClient {
        let addr = stream.peer_addr().expect("could not get peer address");
        NetClient {
            lines: Framed::new(stream, LinesCodec::new()),
            addr,
        }
    }

    pub fn get_addr(&self) -> SocketAddr {
        self.addr
    }

    pub async fn send_line(&mut self, line: &str) -> Result<(), LinesCodecError> {
        self.lines.send(line).await
    }
}

impl Stream for NetClient {
    type Item = Result<String, LinesCodecError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.lines).poll_next(cx)
    }
}