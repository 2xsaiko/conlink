use std::io;

use futures::task::Context;
use tokio::io::{AsyncWriteExt, Stdin, Stdout};
use tokio::macros::support::{Pin, Poll};
use tokio::prelude::AsyncRead;

pub struct TermClient {
    stdin: Stdin,
    stdout: Stdout,
}

impl TermClient {
    pub fn new() -> Self {
        TermClient {
            stdin: tokio::io::stdin(),
            stdout: tokio::io::stdout(),
        }
    }

    pub async fn send_line(&mut self, line: &[u8]) -> io::Result<()> {
        self.stdout.write_all(line).await
    }
}

impl AsyncRead for TermClient {
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.stdin).poll_read(cx, buf)
    }
}