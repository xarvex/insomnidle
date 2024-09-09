use anyhow::Result;

pub mod dbus;
pub mod wayland;

pub trait Inhibitor {
    async fn inhibit(&mut self) -> Result<()>;
    async fn uninhibit(&mut self) -> Result<()>;
}
