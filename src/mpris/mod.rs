use std::sync::Arc;

use futures::future::join_all;

use zbus::{Connection, fdo::DBusProxy};

use crate::Player;

mod player_stream;
pub use player_stream::{BusEvent, PlayerStream};

pub mod event_loop;
pub use event_loop::*;

#[derive(Debug, Clone)]
/// Provides a convenient way to connect to the dbus and retrieve the MPRIS players.
///
/// The connection is ARC counted, so as long as some player object,
/// or the connection isn't dropped, MPRIS events will be received.
pub struct Mpris<'a> {
    connection: Connection,
    pub(crate) proxy: DBusProxy<'a>,
}

impl<'a> Mpris<'a> {
    /// Creates a new connection
    pub async fn new() -> Result<Self, zbus::Error> {
        let connection = Connection::session().await?;
        let proxy = zbus::fdo::DBusProxy::new(&connection).await?;

        Ok(Self { connection, proxy })
    }

    /// Creates a new instance from an already existing DBus connection
    pub async fn new_from_connection(connection: Connection) -> Result<Self, zbus::Error> {
        let proxy = zbus::fdo::DBusProxy::new(&connection).await?;

        Ok(Self { connection, proxy })
    }

    /// Returns a copy of the underlying connection
    pub fn connection(&self) -> Connection {
        self.connection.clone()
    }

    /// Gets all currently available players.
    pub async fn get_players(&self) -> Result<Vec<Arc<Player>>, zbus::Error> {
        let names = self.proxy.list_names().await?;

        Ok(join_all(
            names
                .iter()
                .filter(|name| name.starts_with("org.mpris.MediaPlayer2"))
                .map(async |name| Player::new(name.clone(), self.connection.clone()).await),
        )
        .await
        .into_iter()
        .try_fold(Vec::new(), |mut vec, player| match player {
            Ok(v) => {
                vec.push(Arc::new(v));
                Ok(vec)
            }
            Err(e) => return Err(e),
        })?)
    }
}
