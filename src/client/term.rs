use futures::task::Context;
use tokio::io::Stdin;
use tokio::macros::support::{Pin, Poll};
use tokio::stream::Stream;
use tokio_util::codec::{Framed, FramedRead, LinesCodec, LinesCodecError};

pub struct TermClient {
    stdin: FramedRead<Stdin, LinesCodec>,
}

impl TermClient {
    pub fn new() -> Self {
        TermClient {
            stdin: FramedRead::new(tokio::io::stdin(), LinesCodec::new()),
        }
    }

    pub fn send_line(&mut self, line: &str) {
        println!("{}", line)
    }
}

impl Stream for TermClient {
    type Item = Result<String, LinesCodecError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // Secondly poll the `Framed` stream.
        let result: Option<_> = futures::ready!(Pin::new(&mut self.stdin).poll_next(cx));

        Poll::Ready(result)
    }
}