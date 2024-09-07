use std::io;

use anyhow::{Context, Result};
use clap::Parser;
use tokio::net::UnixStream;
use unidled_ipc::IpcRequest;

#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    #[command(subcommand)]
    command: IpcRequest,
}

async fn send_request(stream: &UnixStream, request: &IpcRequest) -> Result<()> {
    let mut buf = bitcode::encode(request);
    // Probably not ideal, if buffer has newlines.
    buf.push(b'\n');

    stream.writable().await?;

    // Push through any `WouldBlock`.
    loop {
        match stream.try_write(&buf) {
            Ok(_) => {
                break;
            }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                continue;
            }
            Err(e) => {
                return Err(e.into());
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let socket = unidled_ipc::socket();
    let stream = UnixStream::connect(socket)
        .await
        .with_context(|| format!("Failed connecting to daemon socket at {}", socket.display()))?;

    send_request(&stream, &cli.command)
        .await
        .context("Failed sending request to daemon")?;

    Ok(())
}
