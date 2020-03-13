use std::fmt::{Display, Formatter};
use std::fmt;
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::TcpStream;
use tokio::process::ChildStdin;
use tokio::sync::{mpsc, Mutex};

use async_trait::async_trait;

// TODO: abstract this out so that not two versions of the same code are needed
pub mod str;
pub mod bin;

pub type Tx<T> = mpsc::Sender<T>;

pub type Rx<T> = mpsc::Receiver<T>;

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

#[async_trait]
pub trait Client<S> where S: Shared {
    async fn new_term(state: Arc<Mutex<S>>) -> Self;

    async fn new_net(stream: TcpStream, state: Arc<Mutex<S>>) -> Self;

    async fn process(self) -> Result<(), Box<dyn std::error::Error>>;
}

#[async_trait]
pub trait Shared {
    type Data: ?Sized;

    fn new(stdin: ChildStdin) -> Self;

    /// Send a line of text to the program's input.
    async fn write_to_stdin(&mut self, line: &Self::Data);

    /// Send a line of text to all connected clients.
    async fn write_output(&mut self, line: &Self::Data);
}