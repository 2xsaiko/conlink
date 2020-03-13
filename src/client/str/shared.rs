use std::collections::HashMap;

use futures::SinkExt;
use tokio::process::ChildStdin;
use tokio_util::codec::{FramedWrite, LinesCodec};

use async_trait::async_trait;

use crate::client::ClientRef;

use super::Tx;

#[derive(Debug)]
pub enum Message {
    /// A message containing a line of text to be sent to the program.
    ToProgram(String),

    /// A message containing a line of text to be send to connected clients.
    FromProgram(String),
}

/// The state shared between all tasks.
pub struct Shared {
    clients: HashMap<ClientRef, Tx>,
    stdin: FramedWrite<ChildStdin, LinesCodec>,
}

impl Shared {
    pub fn clients(&self) -> &HashMap<ClientRef, Tx> { &self.clients }

    pub fn clients_mut(&mut self) -> &mut HashMap<ClientRef, Tx> { &mut self.clients }
}

#[async_trait]
impl crate::client::Shared for Shared {
    type Data = str;

    /// Create a new shared state.
    fn new(stdin: ChildStdin) -> Self {
        Shared {
            clients: HashMap::new(),
            stdin: FramedWrite::new(stdin, LinesCodec::new()),
        }
    }

    /// Send a line of text to the program's input.
    async fn write_to_stdin(&mut self, line: &Self::Data) {
        match self.stdin.send(line).await {
            Ok(_) => {}
            Err(e) => {
                eprintln!("failed to pass to program: {:?}", e);
            }
        }
    }

    /// Send a line of text to all connected clients.
    async fn write_output(&mut self, line: &Self::Data) {
        for stream in self.clients.values_mut() {
            // don't care about errors, the output will be removed from the clients map if it's
            // disconnected at some point, and the only error that can be returned here is
            // disconnected pipe
            let _ = stream.send(line.to_owned()).await;
        }
    }
}