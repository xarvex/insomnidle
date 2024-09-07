use std::{error::Error, io};

use clap::Parser;
use tokio::net::UnixStream;
use unidled_ipc::IpcRequest;

#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    #[command(subcommand)]
    command: IpcRequest,
}

async fn send_request(stream: &UnixStream, request: &IpcRequest) -> Result<(), Box<dyn Error>> {
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
async fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    let stream = UnixStream::connect(unidled_ipc::socket()).await.unwrap();

    send_request(&stream, &cli.command).await?;

    Ok(())
}
