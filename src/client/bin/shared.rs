use std::collections::HashMap;

use tokio::io::AsyncWriteExt;
use tokio::process::ChildStdin;

use async_trait::async_trait;

use crate::client::ClientRef;

use super::Tx;

/// The state shared between all tasks.
pub struct Shared {
    clients: HashMap<ClientRef, Tx>,
    stdin: ChildStdin,
    echo: bool,
}

impl Shared {
    pub fn clients(&self) -> &HashMap<ClientRef, Tx> { &self.clients }

    pub fn clients_mut(&mut self) -> &mut HashMap<ClientRef, Tx> { &mut self.clients }
}

#[async_trait]
impl crate::client::Shared for Shared {
    type Data = [u8];

    /// Create a new shared state.
    fn new(stdin: ChildStdin, echo: bool) -> Self {
        Shared {
            clients: HashMap::new(),
            stdin,
            echo,
        }
    }

    /// Send a buffer to the program's input.
    async fn write_to_stdin(&mut self, line: &Self::Data, from: ClientRef) {
        match self.stdin.write_all(line).await {
            Ok(_) => {
                if self.echo {
                    for (&r, stream) in self.clients.iter_mut() {
                        if r != from {
                            let _ = stream.send(line.to_owned()).await;
                        }
                    }
                }
            }
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