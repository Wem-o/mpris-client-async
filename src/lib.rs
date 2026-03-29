/// A player, and related stuff
mod player;
pub use player::{Loop, Metadata, Playback, Player, properties, signals, streams};

mod mpris;
pub use mpris::{Mpris, PlayerEvent};

pub use zbus::Error;
