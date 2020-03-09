use std::fmt::{Display, Error, Formatter};
use std::fmt;
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

pub type Tx = mpsc::UnboundedSender<String>;

pub type Rx = mpsc::UnboundedReceiver<String>;

pub struct Client {
    state: Arc<Mutex<Shared>>,
    rx: Rx,
    inner: ClientImpl,
}

pub enum ClientImpl {
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
    pub async fn new_term(state: Arc<Mutex<Shared>>) -> Self {
        Client::new(ClientImpl::Term(TermClient::new()), state).await
    }

    pub async fn new_net(stream: TcpStream, state: Arc<Mutex<Shared>>) -> Self {
        Client::new(ClientImpl::Net(NetClient::new(stream).await), state).await
    }

    pub async fn new(inner: ClientImpl, state: Arc<Mutex<Shared>>) -> Self {
        // Create a channel for this peer
        let (tx, rx) = mpsc::unbounded_channel();

        // Add an entry for this `Peer` in the shared state map.
        state.lock().await.clients.insert(inner.get_ref(), tx);

        Client { inner, rx, state }
    }

    pub async fn process(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        while let Some(result) = self.next().await {
            match result {
                // A message was received from the current user, we should
                // broadcast this message to the other users.
                Ok(Message::ToProgram(msg)) => {
                    let mut state = self.state.lock().await;

                    state.write_to_stdin(&msg).await;
                }
                // A message was received from a peer. Send it to the
                // current user.
                Ok(Message::FromProgram(msg)) => {
                    self.inner.send_line(&msg).await?;
                }
                Err(e) => {
                    println!(
                        "an error occurred while processing messages for {}; error = {:?}",
                        self.inner.get_ref(), e
                    );
                }
            }
        }

        Ok(())
    }
}

// Peer implements `Stream` in a way that polls both the `Rx`, and `Framed` types.
// A message is produced whenever an event is ready until the `Framed` stream returns `None`.
impl Stream for Client {
    type Item = Result<Message, LinesCodecError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // First poll the `UnboundedReceiver`.

        if let Poll::Ready(Some(v)) = Pin::new(&mut self.rx).poll_next(cx) {
            return Poll::Ready(Some(Ok(Message::FromProgram(v))));
        }

        // Secondly poll the `Framed` stream.
        let result: Option<_> = futures::ready!(Pin::new(&mut self.inner).poll_next(cx));

        Poll::Ready(match result {
            // We've received a message we should broadcast to others.
            Some(Ok(message)) => Some(Ok(Message::ToProgram(message))),

            // An error occurred.
            Some(Err(e)) => Some(Err(e)),

            // The stream has been exhausted.
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