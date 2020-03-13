use futures::SinkExt;
use futures::task::Context;
use tokio::io::{Stdin, Stdout};
use tokio::macros::support::{Pin, Poll};
use tokio::stream::Stream;
use tokio_util::codec::{FramedRead, FramedWrite, LinesCodec, LinesCodecError};

pub struct TermClient {
    stdin: FramedRead<Stdin, LinesCodec>,
    stdout: FramedWrite<Stdout, LinesCodec>,
}

impl TermClient {
    pub fn new() -> Self {
        TermClient {
            stdin: FramedRead::new(tokio::io::stdin(), LinesCodec::new()),
            stdout: FramedWrite::new(tokio::io::stdout(), LinesCodec::new()),
        }
    }

    pub async fn send_line(&mut self, line: &str) -> Result<(), LinesCodecError> {
        self.stdout.send(line).await
    }
}

impl Stream for TermClient {
    type Item = Result<String, LinesCodecError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.stdin).poll_next(cx)
    }
}