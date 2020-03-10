use std::io;
use std::io::ErrorKind;

use futures::task::Context;
use tokio::macros::support::{Pin, Poll};
use tokio::prelude::AsyncRead;
use tokio::stream::Stream;

pub struct StreamWrapper<T>
    where T: AsyncRead + Unpin {
    inner: T,
}

impl<T> StreamWrapper<T>
    where T: AsyncRead + Unpin {
    pub fn of(inner: T) -> Self {
        StreamWrapper { inner }
    }

    pub fn into_inner(self) -> T { self.inner }
}

impl<T> Stream for StreamWrapper<T>
    where T: AsyncRead + Unpin {
    type Item = io::Result<Vec<u8>>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut buf = Vec::new();
        let f: io::Result<usize> = futures::ready!(Pin::new(&mut self.inner).poll_read_buf(cx, &mut buf));

        match f {
            Ok(_size) => Poll::Ready(Some(Ok(buf))),
            Err(e)
            if e.kind() == ErrorKind::BrokenPipe || e.kind() == ErrorKind::ConnectionReset => Poll::Ready(None),
            Err(e) => Poll::Ready(Some(Err(e))),
        }
    }
}
