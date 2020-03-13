use std::net::IpAddr;
use std::ops::Deref;
use std::sync::Arc;

use clap::{crate_authors, crate_description, crate_name, crate_version};
use clap::app_from_crate;
use clap::Arg;
use tokio::net::TcpListener;
use tokio::process::{Child, ChildStderr, ChildStdout};
use tokio::stream::StreamExt;
use tokio::sync::Mutex;

use client::bin::Client as BinClient;
use client::bin::shared::Shared as BinShared;
use client::str::Client as StrClient;
use client::str::shared::Shared as StrShared;

use crate::client::{Client, Shared};
use crate::cmd::{BinReadWrapper, ReadWrapper, StrReadWrapper};

mod client;
mod cmd;
mod asyncreadwrap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // get project to recompile if Cargo.toml changes
    include_str!("../Cargo.toml");

    let matches = app_from_crate!()
        .arg(Arg::with_name("port").short("p").long("port").default_value("1337").help("The port to bind the socket to"))
        .arg(Arg::with_name("host").short("H").long("host").default_value("0.0.0.0").help("The host to bind the socket to"))
        .arg(Arg::with_name("quiet").short("q").long("quiet").help("Disable passthrough of command output/input to stdout/stdin"))
        .arg(Arg::with_name("binary").short("b").long("binary").help("Enable binary mode"))
        .arg(Arg::with_name("command").last(true).required(true).multiple(true).help("The command to run"))
        .get_matches();

    let port: u16 = matches.value_of("port").unwrap().parse().expect("invalid port");
    let host: IpAddr = matches.value_of("host").unwrap().parse().expect("invalid target IP address");
    let quiet = matches.is_present("quiet");
    let binary = matches.is_present("binary");
    let command = matches.values_of_lossy("command").unwrap();

    // TODO: find a way to cleanly exit?
    std::process::exit(start(&command, host, port, quiet, binary).await?)
}

async fn start(command: &[String], host: IpAddr, port: u16, quiet: bool, binary: bool) -> Result<i32, Box<dyn std::error::Error>> {
    let mut listener = TcpListener::bind((host, port)).await?;

    let mut child = cmd::start_command(&command)?;

    if binary {
        actually_start::<BinShared, BinClient, BinReadWrapper, Vec<u8>>(listener, &mut child, quiet, BinReadWrapper).await
    } else {
        actually_start::<StrShared, StrClient, StrReadWrapper, String>(listener, &mut child, quiet, StrReadWrapper).await
    }

    Ok(child.await?.code().unwrap_or(126))
}

async fn actually_start<S, C, W, D>(mut listener: TcpListener, child: &mut Child, quiet: bool, wrapper: W)
    where S: Shared<Data=<D as Deref>::Target> + Send + 'static,
          C: Client<S> + Send,
          W: ReadWrapper<ChildStdout, Data=D> + ReadWrapper<ChildStderr, Data=D> + Copy + 'static,
          D: Deref + Send + Sync,
          <D as Deref>::Target: Sync {
    let stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    let state = Arc::new(Mutex::new(S::new(stdin)));

    cmd::process_stdout(stdout, state.clone(), wrapper);
    cmd::process_stdout(stderr, state.clone(), wrapper);

    if !quiet {
        let state = state.clone();
        tokio::spawn(async move {
            let client = C::new_term(state.clone()).await;
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
                            let client = C::new_net(stream, state).await;
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
}
