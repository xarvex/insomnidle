use zbus::{proxy, Connection};

use super::{Inhibitor, Result};

#[proxy(
    default_service = "org.freedesktop.ScreenSaver",
    default_path = "/org/freedesktop/ScreenSaver",
    interface = "org.freedesktop.ScreenSaver"
)]
trait ScreenSaver {
    // zbus method: Inhibit
    async fn inhibit(&self, name: &str, reason: &str) -> zbus::Result<u32>;

    // zbus method: UnInhibit
    async fn un_inhibit(&self, cookie: u32) -> zbus::Result<()>;
}

pub struct DbusInhibitor<'a> {
    _connection: Connection,
    proxy: ScreenSaverProxy<'a>,
    cookie: Option<u32>,
}

impl<'a> DbusInhibitor<'a> {
    pub async fn new() -> Result<DbusInhibitor<'a>> {
        let connection = Connection::session().await?;
        let proxy = ScreenSaverProxy::new(&connection).await?;

        let inhibitor = DbusInhibitor {
            _connection: connection,
            proxy,
            cookie: None,
        };

        Ok(inhibitor)
    }
}

impl Inhibitor for DbusInhibitor<'_> {
    async fn inhibit(&mut self) -> Result<()> {
        if self.cookie.is_none() {
            self.cookie = Some(
                self.proxy
                    .inhibit("insomnidle", "Idle inhibitor was enabled")
                    .await?,
            );
            eprintln!("Inhibiting with cookie {:?}", self.cookie);
        }

        Ok(())
    }

    async fn uninhibit(&mut self) -> Result<()> {
        if let Some(cookie) = self.cookie.take() {
            self.proxy.un_inhibit(cookie).await?;
        }

        Ok(())
    }
}
