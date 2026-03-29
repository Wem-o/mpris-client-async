use std::{collections::HashMap, sync::Arc};

use futures::stream::{self, Stream, StreamExt as _};
use zbus::names::OwnedBusName;

use crate::Player;

use super::Mpris;

// The contents of this file was vibecoded, as it seemed boring :)

/// An MPRIS player has appeared on or disappeared from the session bus.
#[derive(Debug, Clone)]
pub enum PlayerEvent {
    /// A new MPRIS player registered itself on the bus.
    Connected(Arc<Player>),
    /// An MPRIS player that was previously connected has left the bus.
    /// <br>The `Arc<Player>` is the last handle that was kept for that player;
    /// method calls on it will now fail, but its metadata (name, etc.) is
    /// still readable.
    Disconnected(Arc<Player>),
}

impl Mpris<'_> {
    /// Returns a [`Stream`] that yields a [`PlayerEvent`] every time an MPRIS
    /// player connects to or disconnects from the session bus.
    ///
    /// The stream first snapshots every player that is **already** online (so
    /// you do not need a separate `get_players` call to bootstrap state), and
    /// then watches for future arrivals / departures via the
    /// `NameOwnerChanged` D-Bus signal.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use futures::StreamExt as _;
    ///
    /// let mpris = Mpris::new().await?;
    /// let mut events = mpris.player_stream().await?;
    ///
    /// while let Some(event) = events.next().await {
    ///     match event {
    ///         PlayerEvent::Connected(player)    => println!("+ {}", player.name()),
    ///         PlayerEvent::Disconnected(player) => println!("- {}", player.name()),
    ///     }
    /// }
    /// ```
    pub async fn player_stream(&self) -> Result<impl Stream<Item = PlayerEvent>, zbus::Error> {
        // Subscribe first to not miss the first while awawiting for get_players
        let signal_stream = self.proxy.receive_name_owner_changed().await?;

        let known: HashMap<OwnedBusName, Arc<Player>> = self
            .get_players()
            .await?
            .into_iter()
            .map(|p| (p.dbus_name().clone(), p))
            .collect();

        let connection = self.connection.clone();

        let s = stream::unfold(
            (signal_stream, known, connection),
            |(mut signal_stream, mut known, connection)| async move {
                // Loop until we find an event we actually want to surface.
                loop {
                    // If the underlying signal stream ends the bus is gone.
                    let signal = signal_stream.next().await?;

                    let args = match signal.args() {
                        Ok(a) => a,
                        Err(_) => continue,
                    };

                    // Only care about MPRIS names.
                    let name = args.name.to_string();
                    if !name.starts_with("org.mpris.MediaPlayer2") {
                        continue;
                    }

                    // `Optional<UniqueName>` derefs to `Option<UniqueName>`.
                    let had_owner = args.old_owner.is_some();
                    let has_owner = args.new_owner.is_some();

                    match (had_owner, has_owner) {
                        // No previous owner → new owner: player just arrived.
                        (false, true) => {
                            let bus_name: OwnedBusName = match name.as_str().try_into() {
                                Ok(n) => n,
                                Err(_) => continue,
                            };

                            match Player::new(bus_name.clone(), connection.clone()).await {
                                Ok(player) => {
                                    let player = Arc::new(player);
                                    known.insert(bus_name, player.clone());
                                    let state = (signal_stream, known, connection);
                                    return Some((PlayerEvent::Connected(player), state));
                                }
                                Err(_) => continue,
                            }
                        }

                        // Had an owner → no new owner: player just left.
                        (true, false) => {
                            let bus_name: OwnedBusName = match name.as_str().try_into() {
                                Ok(n) => n,
                                Err(_) => continue,
                            };

                            // Return the Arc we were holding so the caller
                            // can still read its metadata.
                            if let Some(player) = known.remove(&bus_name) {
                                let state = (signal_stream, known, connection);
                                return Some((PlayerEvent::Disconnected(player), state));
                            }
                            // Unknown player left (wasn't in our snapshot) — skip.
                            continue;
                        }

                        // Owner changed but didn't appear/disappear (e.g. hand-off).
                        _ => continue,
                    }
                }
            },
        );

        Ok(s)
    }
}
