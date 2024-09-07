use std::{process::ExitCode, result};

use clap::Parser;
use inhibitor::{dbus::DbusInhibitor, Inhibitor};
use tokio::{
    fs,
    io::{AsyncBufReadExt, AsyncReadExt},
    net::{UnixListener, UnixStream},
    signal::unix::{signal, SignalKind},
};
use unidled_ipc::IpcRequest;

mod error;
mod inhibitor;

type Error = error::Error;
type Result<T> = result::Result<T, Error>;

#[derive(Parser)]
#[command(author, version, about)]
struct Cli {}

async fn quit<I: Inhibitor>(inhibitor: &mut I) -> ExitCode {
    match tokio::join!(
        inhibitor.uninhibit(),
        fs::remove_file(unidled_ipc::socket()),
    ) {
        (Ok(()), Ok(())) => ExitCode::SUCCESS,
        (_, _) => ExitCode::FAILURE,
    }
}

// Separate into function because formatting in macros is weird.
async fn answer_stream<I: Inhibitor>(
    mut stream: UnixStream,
    inhibitor: &mut I,
) -> Option<ExitCode> {
    let mut buf = Vec::new();

    match stream.read_to_end(&mut buf).await {
        Ok(_) => {
            // Is there a better way than line-delimited?
            let mut lines = buf.lines();
            loop {
                match lines.next_line().await {
                    Ok(Some(line)) => {
                        let data: IpcRequest = bitcode::decode(line.as_bytes()).unwrap();

                        match data {
                            IpcRequest::Status => println!("ready"),
                            IpcRequest::Inhibit => {
                                inhibitor.inhibit().await;
                            }
                            IpcRequest::Uninhibit => {
                                inhibitor.uninhibit().await;
                            }
                            IpcRequest::Kill => return Some(quit(inhibitor).await),
                        }
                    }
                    Ok(None) => {
                        break;
                    }
                    Err(e) => {
                        eprintln!("{:?}", e);
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("{:?}", e);
        }
    };

    None
}

#[tokio::main]
async fn main() -> ExitCode {
    let _cli = Cli::parse();

    let listener = UnixListener::bind(unidled_ipc::socket()).unwrap();

    let mut inhibitor = DbusInhibitor::new().await.unwrap();

    let mut sighup = signal(SignalKind::hangup()).unwrap();
    let mut sigint = signal(SignalKind::interrupt()).unwrap();
    let mut sigquit = signal(SignalKind::quit()).unwrap();
    let mut sigterm = signal(SignalKind::terminate()).unwrap();

    loop {
        tokio::select! {
            Ok((stream, _)) = listener.accept() => {
                if let Some(code) = answer_stream(stream, &mut inhibitor).await {
                    return code;
                }
            },
            _ = sighup.recv() => return quit(&mut inhibitor).await,
            _ = sigint.recv() => return quit(&mut inhibitor).await,
            _ = sigquit.recv() => return quit(&mut inhibitor).await,
            _ = sigterm.recv() => return quit(&mut inhibitor).await,
        }
    }
}
