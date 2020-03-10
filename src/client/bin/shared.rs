use std::collections::HashMap;

use tokio::io::AsyncWriteExt;
use tokio::process::ChildStdin;

use async_trait::async_trait;

use crate::client::ClientRef;

use super::Tx;

#[derive(Debug)]
pub enum Message {
    /// A message containing a line of text to be sent to the program.
    ToProgram(Vec<u8>),

    /// A message containing a line of text to be send to connected clients.
    FromProgram(Vec<u8>),
}

/// The state shared between all tasks.
pub struct Shared {
    clients: HashMap<ClientRef, Tx>,
    stdin: ChildStdin,
}

impl Shared {
    /// Create a new shared state.
    pub fn new(stdin: ChildStdin) -> Self {
        Shared {
            clients: HashMap::new(),
            stdin,
        }
    }

    pub fn clients(&self) -> &HashMap<ClientRef, Tx> { &self.clients }

    pub fn clients_mut(&mut self) -> &mut HashMap<ClientRef, Tx> { &mut self.clients }
}

#[async_trait]
impl crate::client::Shared for Shared {
    type Data = [u8];

    /// Send a buffer to the program's input.
    async fn write_to_stdin(&mut self, line: &Self::Data) {
        match self.stdin.write_all(line).await {
            Ok(_) => {}
            Err(e) => {
                eprintln!("failed to pass to program: {:?}", e);
            }
        }
    }

    /// Send a buffer to all connected clients.
    async fn write_output(&mut self, line: &Self::Data) {
        for stream in self.clients.values_mut() {
            // don't care about errors, the output will be removed from the clients map if it's
            // disconnected at some point, and the only error that can be returned here is
            // disconnected pipe
            let _ = stream.send(line.to_owned()).await;
        }
    }
}