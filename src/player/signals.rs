//! Types of the signals of a [`Player`](super::Player)

use std::time::Duration;

use zbus::zvariant::DynamicDeserialize;

use crate::player::Interface;

/// A dbus signal, check [`Player::subscribe`](super::Player::subscribe)
pub trait Signal {
    /// Parses form zbus's Value as this, with into_output transformations may be applied
    type ParseAs: serde::de::DeserializeOwned + DynamicDeserialize<'static> + Send + 'static;

    /// The output type of the property
    type Output: Send + 'static;

    /// The name as specified by the [specs](https://specifications.freedesktop.org/mpris/latest/Media_Player.html)
    fn name(&self) -> &'static str;

    /// The interface the property is on.
    fn interface(&self) -> Interface {
        Interface::default()
    }

    /// Convert the parsed value into the final Output
    fn into_output(&self, value: Self::ParseAs) -> Self::Output;
}

pub const SEEKED: Seeked = Seeked;
/// Indicates that the track position has changed in a way that is inconsistant with the current playing state. This could be seeking, pausing the player, or a track change.
/// <br>To follow the current position of the player, you need to either poll the [`Position`](super::properties::Position) every X time, or subscribe to the
/// changes automatically handled by [`PositionStream`](super::PositionStream)
pub struct Seeked;
impl Signal for Seeked {
    type Output = Duration;
    type ParseAs = i64;

    fn name(&self) -> &'static str {
        "Seeked"
    }

    fn interface(&self) -> Interface {
        Interface::Player
    }

    fn into_output(&self, value: Self::ParseAs) -> Self::Output {
        Duration::from_micros(value as u64)
    }
}
