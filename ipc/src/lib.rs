use std::{
    env,
    path::{Path, PathBuf},
    sync::OnceLock,
};

use bitcode::{Decode, Encode};
#[cfg(feature = "clap")]
use clap::Subcommand;

#[derive(Debug, Decode, Encode)]
#[cfg_attr(feature = "clap", derive(Subcommand))]
pub enum IpcRequest {
    Status,
    Inhibit,
    Uninhibit,
    Kill,
}

#[derive(Debug, Decode, Encode)]
pub enum IpcResponse {
    Ok,
    Err,
}

pub fn socket() -> &'static Path {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let runtime = env::var("XDG_RUNTIME_DIR").unwrap();

        let display = match env::var("WAYLAND_DISPLAY") {
            Ok(wayland_socket) => {
                let mut i = 0;
                for (j, ch) in wayland_socket.bytes().enumerate().rev() {
                    if ch == b'/' {
                        i = j + 1;
                        break;
                    }
                }
                (wayland_socket[i..]).to_string()
            }
            Err(_) => "wayland-0".to_string(),
        };

        Path::new(&format!("{runtime}/insomnidle-{display}.sock")).to_path_buf()
    })
}
