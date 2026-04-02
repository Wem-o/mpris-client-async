//! Types of the signals of a [`Player`](super::Player)

use std::{fmt::Debug, hash::Hash};

use zbus::zvariant::DynamicDeserialize;

use crate::player::Interface;

/// A dbus signal, check [`Player::subscribe`](super::Player::subscribe)
pub trait Signal: Debug + Clone + Copy + Hash + PartialEq {
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

pub mod types {
    use std::time::Duration;

    use crate::{player::Interface, signals::Signal};

    /// Indicates that the track position has changed in a way that is inconsistant with the current playing state.
    /// This could be seeking, pausing the player, or a track change.
    ///
    /// It's recommended to use [`PositionStream`](super::super::streams::PositionStream)
    /// that will keep track of this stream, and other factors (like pause / speed).
    #[derive(Debug, Clone, Copy, PartialEq, Hash)]
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
}
