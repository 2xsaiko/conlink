use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::process::Stdio;
use std::sync::Arc;

use clap::{crate_authors, crate_description, crate_name, crate_version};
use clap::app_from_crate;
use clap::Arg;
use futures::SinkExt;
use futures::task::Context;
use tokio::macros::support::{Pin, Poll};
use tokio::net::{TcpListener, TcpStream};
use tokio::prelude::*;
use tokio::process::{Child, ChildStdin, Command};
use tokio::stream::{Stream, StreamExt};
use tokio::sync::{mpsc, Mutex};
use tokio_util::codec::{Framed, FramedRead, FramedWrite, LinesCodec, LinesCodecError};

use crate::client::{Client, ClientRef, Rx, Tx};

mod client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = app_from_crate!()
        .arg(Arg::with_name("port").short("p").long("port").default_value("1337").help("The port to bind the socket to"))
        .arg(Arg::with_name("host").short("H").long("host").default_value("0.0.0.0").help("The host to bind the socket to"))
        .arg(Arg::with_name("quiet").short("q").long("quiet").help("Disable printing program output to stdout"))
        .arg(Arg::with_name("command").last(true).required(true).multiple(true).help("The command to run"))
        .get_matches();

    let port: u16 = matches.value_of("port").unwrap().parse().expect("invalid port");
    let host: IpAddr = matches.value_of("host").unwrap().parse().expect("invalid target IP address");
    let command = matches.values_of_lossy("command").unwrap();

    let mut listener = TcpListener::bind((host, port)).await?;

    let mut child = start_command(&command)?;

    let stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    let state = Arc::new(Mutex::new(Shared::new(stdin)));

    process_stdout(stdout, state.clone());
    process_stdout(stderr, state.clone());

    {
        let state = state.clone();
        tokio::spawn(async move {
            let mut client = Client::new_term(state.clone()).await;
            if let Err(e) = client.process().await {
                eprintln!("{:?}", e);
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
                            let mut client = Client::new_net(stream, state).await;
                            if let Err(e) = client.process().await {
                                eprintln!("{:?}", e);
                            }
                        });
                    }
                    Err(e) => eprintln!("failed to accept connection: {:?}", e),
                }
            }
        });
    }

    let r = child.await?;

    std::process::exit(r.code().unwrap_or(126))
}

fn process_stdout(stdout: impl AsyncRead + Unpin + Send + 'static, state: Arc<Mutex<Shared>>) {
    tokio::spawn(async move {
        let mut reader = FramedRead::new(stdout, LinesCodec::new());
        while let Some(line) = reader.next().await {
            match line {
                Ok(line) => {
                    state.lock().await.write_output(&line).await;
                }
                Err(e) => {
                    eprintln!("{:?}", e);
                }
            }
        }
    });
}

fn start_command(command: &[String]) -> io::Result<Child> {
    Command::new(command.first().unwrap())
        .args(&command[1..])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
}

pub struct Shared {
    clients: HashMap<ClientRef, Tx>,
    stdin: FramedWrite<ChildStdin, LinesCodec>,
}

#[derive(Debug)]
pub enum Message {
    ToProgram(String),
    FromProgram(String),
}

impl Shared {
    pub fn new(stdin: ChildStdin) -> Self {
        Shared {
            clients: HashMap::new(),
            stdin: FramedWrite::new(stdin, LinesCodec::new()),
        }
    }

    pub async fn write_to_stdin(&mut self, line: &str) {
        match self.stdin.send(line).await {
            Ok(_) => {}
            Err(e) => {
                eprintln!("failed to pass to program: {:?}", e);
            }
        }
    }

    pub async fn write_output(&mut self, line: &str) {
        for stream in self.clients.values() {
            match stream.send(line.to_owned()) {
                Ok(_) => {}
                Err(e) => eprintln!("{:?}", e),
            }
        }
    }
}
