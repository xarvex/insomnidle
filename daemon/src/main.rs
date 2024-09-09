use std::process::ExitCode;

use anyhow::{Context, Error, Result};
use clap::Parser;
use inhibitor::{dbus::DbusInhibitor, Inhibitor};
use insomnidle_ipc::IpcRequest;
use tokio::{
    fs,
    io::{AsyncBufReadExt, AsyncReadExt},
    net::{UnixListener, UnixStream},
    signal::unix::{signal, Signal, SignalKind},
};

mod inhibitor;

#[derive(Parser)]
#[command(author, version, about)]
struct Cli {}

async fn quit<I: Inhibitor>(inhibitor: &mut I) -> ExitCode {
    let socket = insomnidle_ipc::socket();
    let result1 = fs::remove_file(socket)
        .await
        .with_context(|| format!("Could not remove socket at {}", socket.display()));
    if let Err(ref e) = result1 {
        eprintln!("{:?}", e);
    }

    let result2 = inhibitor.uninhibit().await.context("Could not uninhibit");
    if let Err(ref e) = result2 {
        eprintln!("{:?}", e);
    }

    if result1.is_err() || result2.is_err() {
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

// Separate into function because formatting in macros is weird.
async fn answer_stream<I: Inhibitor>(mut stream: UnixStream, inhibitor: &mut I) -> bool {
    let mut buf = Vec::new();

    match stream.read_to_end(&mut buf).await {
        Ok(_) => {
            // Is there a better way than line-delimited?
            let mut lines = buf.lines();
            loop {
                match lines.next_line().await {
                    Ok(Some(line)) => {
                        let result: Result<IpcRequest> =
                            bitcode::decode(line.as_bytes()).context("Could not decode message");

                        // TODO: responses
                        match result {
                            Ok(IpcRequest::Status) => {}
                            Ok(IpcRequest::Inhibit) => {
                                let result = inhibitor.inhibit().await.context("Could not inhibit");
                                if let Err(ref e) = result {
                                    eprintln!("{:?}", e);
                                }
                            }
                            Ok(IpcRequest::Uninhibit) => {
                                let result =
                                    inhibitor.uninhibit().await.context("Could not uninhibit");
                                if let Err(ref e) = result {
                                    eprintln!("{:?}", e);
                                }
                            }
                            Ok(IpcRequest::Kill) => return false,
                            Err(e) => {
                                eprintln!("{:?}", e);
                            }
                        }
                    }
                    Ok(None) => break,
                    Err(e) => {
                        let e = Error::from(e).context("Could not read line for socket stream");
                        eprintln!("{:?}", e);
                    }
                }
            }
        }
        Err(e) => {
            let e = Error::from(e).context("Could not read socket stream");
            eprintln!("{:?}", e);
        }
    };

    true
}

fn register_signal(signal_name: &str, signal_kind: SignalKind) -> Option<Signal> {
    signal(signal_kind)
        .with_context(|| format!("Could not register handler for {}", signal_name))
        .inspect_err(|e| eprintln!("{:?}", e))
        .ok()
}

async fn optional_signal(signal: Option<&mut Signal>) -> Option<()> {
    match signal {
        Some(s) => s.recv().await,
        None => None,
    }
}

#[tokio::main]
async fn main() -> Result<ExitCode> {
    let _cli = Cli::parse();

    let socket = insomnidle_ipc::socket();
    let listener = UnixListener::bind(socket)
        .with_context(|| format!("Failed establishing socket at {}", socket.display()))?;

    let mut inhibitor = DbusInhibitor::new()
        .await
        .context("Failed establishing D-Bus inhibitor")?;

    let mut sighup = register_signal("SIGHUP", SignalKind::hangup());
    let mut sigint = register_signal("SIGINT", SignalKind::interrupt());
    let mut sigquit = register_signal("SIGQUIT", SignalKind::quit());
    let mut sigterm = register_signal("SIGTERM", SignalKind::terminate());

    loop {
        tokio::select! {
            Ok((stream, _)) = listener.accept() => {
                if !answer_stream(stream, &mut inhibitor).await {
                    return Ok(quit(&mut inhibitor).await);
                }
            },
            _ = optional_signal(sighup.as_mut()) => return Ok(quit(&mut inhibitor).await),
            _ = optional_signal(sigint.as_mut()) => return Ok(quit(&mut inhibitor).await),
            _ = optional_signal(sigquit.as_mut()) => return Ok(quit(&mut inhibitor).await),
            _ = optional_signal(sigterm.as_mut()) => return Ok(quit(&mut inhibitor).await),
        }
    }
}
