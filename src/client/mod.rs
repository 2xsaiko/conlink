use std::fmt::{Display, Formatter};
use std::fmt;
use std::io::ErrorKind;
use std::net::SocketAddr;
use std::sync::Arc;

use futures::task::Context;
use tokio::macros::support::{Pin, Poll};
use tokio::net::TcpStream;
use tokio::stream::{Stream, StreamExt};
use tokio::sync::{mpsc, Mutex};
use tokio_util::codec::LinesCodecError;

use crate::{Message, Shared};
use crate::client::net::NetClient;
use crate::client::term::TermClient;

pub mod term;
pub mod net;

pub type Tx = mpsc::Sender<String>;

pub type Rx = mpsc::Receiver<String>;

/// A client connected to the running program.
/// In most cases, this is connected through the network.
pub struct Client {
    state: Arc<Mutex<Shared>>,
    rx: Rx,
    inner: ClientImpl,
}

enum ClientImpl {
    Term(TermClient),
    Net(NetClient),
}

#[derive(Debug, Hash, Eq, PartialEq)]
pub enum ClientRef {
    Term,
    Net(SocketAddr),
}

impl Display for ClientRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ClientRef::Term => write!(f, "<stdout>"),
            ClientRef::Net(a) => write!(f, "{}", a),
        }
    }
}

impl Client {
    /// Create a new passthrough client connecting the running program to stdout/stdin.
    pub async fn new_term(state: Arc<Mutex<Shared>>) -> Self {
        Client::new(ClientImpl::Term(TermClient::new()), state).await
    }

    /// Create a new client connected to a TCP stream.
    pub async fn new_net(stream: TcpStream, state: Arc<Mutex<Shared>>) -> Self {
        Client::new(ClientImpl::Net(NetClient::new(stream).await), state).await
    }

    async fn new(inner: ClientImpl, state: Arc<Mutex<Shared>>) -> Self {
        let (tx, rx) = mpsc::channel(4096);

        state.lock().await.clients.insert(inner.get_ref(), tx);

        Client { inner, rx, state }
    }

    /// Start processing the client. This consumes the client after the connection to it has closed.
    pub async fn process(mut self) -> Result<(), Box<dyn std::error::Error>> {
        while let Some(result) = self.next().await {
            match result {
                Ok(Message::ToProgram(msg)) => {
                    let mut state = self.state.lock().await;

                    state.write_to_stdin(&msg).await;
                }
                Ok(Message::FromProgram(msg)) => {
                    match self.inner.send_line(&msg).await {
                        Ok(_) => {}
                        // don't print broken pipe/connection reset errors because those will always
                        // occur on disconnection before the stream knows it has to close
                        Err(LinesCodecError::Io(e))
                        if e.kind() == ErrorKind::BrokenPipe || e.kind() == ErrorKind::ConnectionReset => {}
                        Err(e) => {
                            Err(e)?
                        }
                    }
                }
                Err(e) => {
                    eprintln!(
                        "an error occurred while processing messages for {}; error = {:?}",
                        self.inner.get_ref(), e
                    );
                }
            }
        }

        Ok(())
    }
}

impl Stream for Client {
    type Item = Result<Message, LinesCodecError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if let Poll::Ready(Some(v)) = Pin::new(&mut self.rx).poll_next(cx) {
            return Poll::Ready(Some(Ok(Message::FromProgram(v))));
        }

        let result: Option<_> = futures::ready!(Pin::new(&mut self.inner).poll_next(cx));

        Poll::Ready(match result {
            Some(Ok(message)) => Some(Ok(Message::ToProgram(message))),
            Some(Err(e)) => Some(Err(e)),
            None => None,
        })
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        let addr = self.inner.get_ref();
        let state = self.state.clone();
        tokio::spawn(async move {
            let mut state = state.lock().await;
            state.clients.remove(&addr);
        });
    }
}

impl ClientImpl {
    fn get_ref(&self) -> ClientRef {
        match self {
            ClientImpl::Term(_) => ClientRef::Term,
            ClientImpl::Net(c) => ClientRef::Net(c.get_addr()),
        }
    }

    async fn send_line(&mut self, line: &str) -> Result<(), LinesCodecError> {
        match self {
            ClientImpl::Term(c) => Ok(c.send_line(line)),
            ClientImpl::Net(c) => c.send_line(line).await,
        }
    }
}

impl Stream for ClientImpl {
    type Item = Result<String, LinesCodecError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.get_mut() {
            ClientImpl::Term(c) => Pin::new(c).poll_next(cx),
            ClientImpl::Net(c) => Pin::new(c).poll_next(cx),
        }
    }
}