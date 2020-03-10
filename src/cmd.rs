use std::process::Stdio;
use std::sync::Arc;

use tokio::prelude::*;
use tokio::process::{Child, Command};
use tokio::stream::StreamExt;
use tokio::sync::Mutex;
use tokio_util::codec::{FramedRead, LinesCodec};

use crate::Shared;

pub fn start_command(command: &[String]) -> io::Result<Child> {
    Command::new(command.first().unwrap())
        .args(&command[1..])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
}

/// Start a task reading the program's output, sending it to the connected clients.
pub fn process_stdout(stdout: impl AsyncRead + Unpin + Send + 'static, state: Arc<Mutex<Shared>>) {
    tokio::spawn(async move {
        let mut reader = FramedRead::new(stdout, LinesCodec::new());
        while let Some(line) = reader.next().await {
            match line {
                Ok(line) => {
                    state.lock().await.write_output(&line).await;
                }
                Err(e) => {
                    eprintln!("failed to read from program output: {:?}", e);
                }
            }
        }
    });
}
