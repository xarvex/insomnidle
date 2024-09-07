use std::result;

pub mod dbus;
pub mod error;
pub mod wayland;

type Error = error::Error;
type Result<T> = result::Result<T, Error>;

pub trait Inhibitor {
    async fn inhibit(&mut self) -> Result<()>;
    async fn uninhibit(&mut self) -> Result<()>;
}
