use std::error::Error;
use std::fmt::Debug;
use std::ops::Deref;
use std::process::Stdio;
use std::sync::Arc;

use tokio::io::{AsyncReadExt, ErrorKind};
use tokio::prelude::*;
use tokio::process::{Child, Command};
use tokio::stream::StreamExt;
use tokio::sync::Mutex;
use tokio_util::codec::{FramedRead, LinesCodec, LinesCodecError};

use async_trait::async_trait;

use crate::asyncreadwrap::StreamWrapper;
use crate::client::Shared;

pub fn start_command(command: &[String]) -> io::Result<Child> {
    Command::new(command.first().unwrap())
        .args(&command[1..])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
}

/// Start a task reading the program's output, sending it to the connected clients.
pub fn process_stdout<T, W, S, D>(stdout: T, state: Arc<Mutex<S>>, rw: W)
    where T: AsyncRead + Unpin + Send + 'static,
          W: ReadWrapper<T, Data=D> + 'static,
          S: Shared<Data=<D as Deref>::Target> + Send + 'static,
          D: Deref + Send + Sync,
          <D as Deref>::Target: Sync {
    tokio::spawn(async move {
        let mut reader = rw.create_reader(stdout);
        while let Some(line) = rw.read(&mut reader).await {
            match line {
                Ok(line) => {
                    state.lock().await.write_output(line.deref()).await;
                }
                Err(e) => {
                    eprintln!("failed to read from program output: {:?}", e);
                }
            }
        }
    });
}

#[async_trait]
pub trait ReadWrapper<T>: Send + Sync
    where T: AsyncRead + Unpin + Send + 'static {
    type Reader: Send;
    type Data;
    type Error: Debug + Send;

    fn create_reader(&self, stdout: T) -> Self::Reader;

    async fn read(&self, reader: &mut Self::Reader) -> Option<Result<Self::Data, Self::Error>>;
}

#[derive(Copy, Clone)]
pub struct StrReadWrapper;

#[async_trait]
impl<T> ReadWrapper<T> for StrReadWrapper
    where T: AsyncRead + Unpin + Send + 'static {
    type Reader = FramedRead<T, LinesCodec>;
    type Data = String;
    type Error = LinesCodecError;

    fn create_reader(&self, stdout: T) -> Self::Reader {
        FramedRead::new(stdout, LinesCodec::new())
    }

    async fn read(&self, reader: &mut Self::Reader) -> Option<Result<Self::Data, LinesCodecError>> {
        match reader.next().await {
            Some(Ok(v)) => Some(Ok(v)),
            Some(Err(e)) => Some(Err(e)),
            None => None,
        }
    }
}

#[derive(Copy, Clone)]
pub struct BinReadWrapper;

#[async_trait]
impl<T> ReadWrapper<T> for BinReadWrapper
    where T: AsyncRead + Unpin + Send + 'static {
    type Reader = T;
    type Data = Vec<u8>;
    type Error = io::Error;

    fn create_reader(&self, stdout: T) -> Self::Reader {
        stdout
    }

    async fn read(&self, reader: &mut Self::Reader) -> Option<Result<Self::Data, io::Error>> {
        StreamWrapper::of(reader).next().await
    }
}