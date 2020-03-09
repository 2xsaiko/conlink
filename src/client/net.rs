use std::net::SocketAddr;

use futures::SinkExt;
use futures::task::Context;
use tokio::macros::support::{Pin, Poll};
use tokio::net::TcpStream;
use tokio::stream::Stream;
use tokio_util::codec::{Framed, LinesCodec, LinesCodecError};

pub struct NetClient {
    lines: Framed<TcpStream, LinesCodec>,
}

impl NetClient {
    pub async fn new(
        stream: TcpStream,
    ) -> NetClient {
        NetClient { lines: Framed::new(stream, LinesCodec::new()) }
    }

    pub fn get_addr(&self) -> SocketAddr {
        self.lines.get_ref().peer_addr().expect("could not get peer address")
    }

    pub async fn send_line(&mut self, line: &str) -> Result<(), LinesCodecError> {
        self.lines.send(line).await
    }
}

impl Stream for NetClient {
    type Item = Result<String, LinesCodecError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // Secondly poll the `Framed` stream.
        let result: Option<_> = futures::ready!(Pin::new(&mut self.lines).poll_next(cx));

        Poll::Ready(result)
    }
}