use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;

use clap::{crate_authors, crate_description, crate_name, crate_version};
use clap::app_from_crate;
use clap::Arg;
use futures::SinkExt;
use tokio::net::TcpListener;
use tokio::process::ChildStdin;
use tokio::stream::StreamExt;
use tokio::sync::Mutex;
use tokio_util::codec::{FramedWrite, LinesCodec};

use crate::client::{Client, ClientRef, Tx};

mod client;
mod cmd;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // get project to recompile if Cargo.toml changes
    include_str!("../Cargo.toml");

    let matches = app_from_crate!()
        .arg(Arg::with_name("port").short("p").long("port").default_value("1337").help("The port to bind the socket to"))
        .arg(Arg::with_name("host").short("H").long("host").default_value("0.0.0.0").help("The host to bind the socket to"))
        .arg(Arg::with_name("quiet").short("q").long("quiet").help("Disable passthrough of command output/input to stdout/stdin"))
        .arg(Arg::with_name("command").last(true).required(true).multiple(true).help("The command to run"))
        .get_matches();

    let port: u16 = matches.value_of("port").unwrap().parse().expect("invalid port");
    let host: IpAddr = matches.value_of("host").unwrap().parse().expect("invalid target IP address");
    let quiet = matches.is_present("quiet");
    let command = matches.values_of_lossy("command").unwrap();

    // TODO: find a way to cleanly exit?
    std::process::exit(start(&command, host, port, quiet).await?)
}

async fn start(command: &[String], host: IpAddr, port: u16, quiet: bool) -> Result<i32, Box<dyn std::error::Error>> {
    let mut listener = TcpListener::bind((host, port)).await?;

    let mut child = cmd::start_command(&command)?;

    let stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    let state = Arc::new(Mutex::new(Shared::new(stdin)));

    cmd::process_stdout(stdout, state.clone());
    cmd::process_stdout(stderr, state.clone());

    if !quiet {
        let state = state.clone();
        tokio::spawn(async move {
            let client = Client::new_term(state.clone()).await;
            if let Err(e) = client.process().await {
                eprintln!("error while processing terminal client: {:?}", e);
            }
        });
    }

    {
        let state = state.clone();
        tokio::spawn(async move {
            while let Some(stream) = listener.next().await {
                match stream {
                    Ok(stream) => {
                        let state = state.clone();
                        tokio::spawn(async move {
                            let client = Client::new_net(stream, state).await;
                            if let Err(e) = client.process().await {
                                eprintln!("error while processing network client: {:?}", e);
                            }
                        });
                    }
                    Err(e) => eprintln!("failed to accept connection: {:?}", e),
                }
            }
        });
    }

    Ok(child.await?.code().unwrap_or(126))
}

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
    /// Create a new shared state.
    pub fn new(stdin: ChildStdin) -> Self {
        Shared {
            clients: HashMap::new(),
            stdin: FramedWrite::new(stdin, LinesCodec::new()),
        }
    }

    /// Send a line of text to the program's input.
    pub async fn write_to_stdin(&mut self, line: &str) {
        match self.stdin.send(line).await {
            Ok(_) => {}
            Err(e) => {
                eprintln!("failed to pass to program: {:?}", e);
            }
        }
    }

    /// Send a line of text to all connected clients.
    pub async fn write_output(&mut self, line: &str) {
        for stream in self.clients.values_mut() {
            // don't care about errors, the output will be removed from the clients map if it's
            // disconnected at some point, and the only error that can be returned here is
            // disconnected pipe
            let _ = stream.send(line.to_owned()).await;
        }
    }
}
