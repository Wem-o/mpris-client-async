//! Types of the properties of a [`Player`](super::Player)

use std::any::Any;
use std::fmt::Debug;
use std::{collections::HashMap, time::Duration};

use zbus::zvariant::OwnedValue;

use crate::player::enums::Interface;
use crate::{Loop, Metadata as Mtd, Playback};

/// Can be used to get some property from the bus.
///
/// Properties also may implement [WritableProperty], or [ControlWritableProperty] (but shouldn't implement both at the same time).
pub trait Property: Debug {
    /// Parses form zbus's Value as this, with into_output transformations may be applied
    type ParseAs: serde::de::DeserializeOwned + Send + 'static + Clone;

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

/// Implementators of this are writable [properties](Property).
///
/// A [Property] should not implement both this and [ControlWritableProperty] at the same time!
pub trait WritableProperty: Property {
    /// The opposite of [Property::into_output], as it converts the [Property::Output] into [Property::ParseAs]
    fn from_output(&self, value: Self::Output) -> Self::ParseAs;
}

/// Implementors are [properties](Property) that can be modified, but only if [CanControl] is true.
/// A [Property] should not implement both this and [WritableProperty] at the same time!
/// <br>According to the specs, this describes the player's implementation, rather than the current state, meaning this wont change after an object is registered.
pub trait ControlWritableProperty: Property {
    /// The opposite of [Property::into_output], as it converts the [Property::Output] into [Property::ParseAs]
    fn from_output(&self, value: Self::Output) -> Self::ParseAs;
}

pub struct AnyStreamYield {
    pub value: Box<dyn Any + Send>,
}

/// An object (or dyn) safe version of [`Property`].
pub trait AnyProperty {
    fn name(&self) -> &'static str;
    fn interface(&self) -> Interface;
}
impl<P> AnyProperty for P
where
    P: Property + Debug + Send + Sync + Unpin + 'static,
{
    fn interface(&self) -> Interface {
        P::interface(&self)
    }

    fn name(&self) -> &'static str {
        P::name(&self)
    }
}

// ======= TYPES =======

pub const CANQUIT: CanQuit = CanQuit;
/// If false, calling Quit will have no effect.
/// <br>If true, calling Quit will cause the media application to <b>attempt</b> to quit
/// (although it may still be prevented from quitting by the user, for example).
#[derive(Debug)]
pub struct CanQuit;
impl Property for CanQuit {
    type Output = bool;
    type ParseAs = bool;

    fn name(&self) -> &'static str {
        "CanQuit"
    }

    fn into_output(&self, value: Self::ParseAs) -> Self::Output {
        value
    }
}

pub const CANCONTROL: CanControl = CanControl;
/// If it's possible to control the some of player's properties. These types implement [ControlWritableProperty]!
/// <br>According to the specs, this describes the player's implementation, rather than the current state, meaning this wont change after an object is registered.
#[derive(Debug)]
pub struct CanControl;
impl Property for CanControl {
    type Output = bool;
    type ParseAs = bool;

    fn interface(&self) -> Interface {
        Interface::Player
    }

    fn into_output(&self, value: Self::ParseAs) -> Self::Output {
        value
    }

    fn name(&self) -> &'static str {
        "CanControl"
    }
}

pub const FULLSCREEN: Fullscreen = Fullscreen;
/// Whether the media player is occupying the fullscreen.
#[derive(Debug)]
pub struct Fullscreen;
impl Property for Fullscreen {
    type Output = bool;
    type ParseAs = bool;

    fn name(&self) -> &'static str {
        "Fullscreen"
    }

    fn into_output(&self, value: Self::ParseAs) -> Self::Output {
        value
    }
}
impl WritableProperty for Fullscreen {
    fn from_output(&self, value: Self::Output) -> Self::ParseAs {
        value.into()
    }
}

pub const CANSETFULLSCREEN: CanSetFullscreen = CanSetFullscreen;
/// If false setting Fullscreen will have no effect
#[derive(Debug)]
pub struct CanSetFullscreen;
impl Property for CanSetFullscreen {
    type Output = bool;
    type ParseAs = bool;

    fn name(&self) -> &'static str {
        "CanSetFullscreen"
    }

    fn into_output(&self, value: Self::ParseAs) -> Self::Output {
        value
    }
}

pub const CANRAISE: CanRaise = CanRaise;
/// If raise() will work.
/// <br>Note: raising is the process of bringing the media player to front, for example maximizing it, or jumping to it in the visual environment.
#[derive(Debug)]
pub struct CanRaise;
impl Property for CanRaise {
    type Output = bool;
    type ParseAs = bool;

    fn name(&self) -> &'static str {
        "CanRaise"
    }

    fn into_output(&self, value: Self::ParseAs) -> Self::Output {
        value
    }
}

pub const HASTRACKLIST: HasTrackList = HasTrackList;
/// If the player has tracklist
#[derive(Debug)]
pub struct HasTrackList;
impl Property for HasTrackList {
    type Output = bool;
    type ParseAs = bool;

    fn name(&self) -> &'static str {
        "HasTrackList"
    }

    fn into_output(&self, value: Self::ParseAs) -> Self::Output {
        value
    }
}

pub const IDENTITY: Identity = Identity;
/// The "display name" of the player. For example "Mozilla Firefox" or "VLC media player"
#[derive(Debug)]
pub struct Identity;
impl Property for Identity {
    type Output = String;
    type ParseAs = String;

    fn name(&self) -> &'static str {
        "Identity"
    }

    fn into_output(&self, value: Self::ParseAs) -> Self::Output {
        value
    }
}

pub const DESKTOPENTRY: DesktopEntry = DesktopEntry;
/// The desktop entry of the player. For example "firefox" or "vlc"
#[derive(Debug)]
pub struct DesktopEntry;
impl Property for DesktopEntry {
    type Output = String;
    type ParseAs = String;

    fn name(&self) -> &'static str {
        "DesktopEntry"
    }

    fn into_output(&self, value: Self::ParseAs) -> Self::Output {
        value
    }
}

pub const SUPPORTEDURIS: SupportedURIs = SupportedURIs;
/// The URI schemes supported by the media player.This can be viewed as protocols supported by the player in almost all cases.
#[derive(Debug)]
pub struct SupportedURIs;
impl Property for SupportedURIs {
    type Output = Vec<String>;
    type ParseAs = Vec<String>;

    fn name(&self) -> &'static str {
        "SupportedUriSchemes"
    }

    fn into_output(&self, value: Self::ParseAs) -> Self::Output {
        value
    }
}

pub const SUPPORTEDMIMES: SupportedMIMEs = SupportedMIMEs;
/// The mime-types supported by the media player.
/// <br>Mime-types should be in the standard format (eg: audio/mpeg or application/ogg).
#[derive(Debug)]
pub struct SupportedMIMEs;
impl Property for SupportedMIMEs {
    type Output = Vec<String>;
    type ParseAs = Vec<String>;

    fn name(&self) -> &'static str {
        "SupportedMimeTypes"
    }

    fn into_output(&self, value: Self::ParseAs) -> Self::Output {
        value
    }
}

pub const PLAYBACKSTATUS: PlaybackStatus = PlaybackStatus;
/// The current playback status, see [super::Playback] for more details
#[derive(Debug)]
pub struct PlaybackStatus;
impl Property for PlaybackStatus {
    type Output = Playback;
    type ParseAs = String;

    fn interface(&self) -> Interface {
        Interface::Player
    }

    fn into_output(&self, value: Self::ParseAs) -> Self::Output {
        value.into()
    }

    fn name(&self) -> &'static str {
        "PlaybackStatus"
    }
}

pub const LOOPSTATUS: LoopStatus = LoopStatus;
/// The current loop / repeat status. See [super::Loop] for more details
#[derive(Debug)]
pub struct LoopStatus;
impl Property for LoopStatus {
    type Output = Loop;
    type ParseAs = String;

    fn interface(&self) -> Interface {
        Interface::Player
    }

    fn into_output(&self, value: Self::ParseAs) -> Self::Output {
        value.into()
    }

    fn name(&self) -> &'static str {
        "LoopStatus"
    }
}
impl ControlWritableProperty for LoopStatus {
    fn from_output(&self, value: Self::Output) -> Self::ParseAs {
        value.to_string()
    }
}

pub const RATE: Rate = Rate;
/// The current playback rate. This allows clients to display (reasonably) accurate progress bars without having to regularly query the media player for the current position.
///
/// <br>The value must fall in the range described by MinimumRate and MaximumRate, and must not be 0.0.
/// If playback is paused, the PlaybackStatus property should be used to indicate this. A value of 0.0 should not be set by the client.
/// If it is, the media player should act as though Pause was called.
///
/// <br>If the media player has no ability to play at speeds other than the normal playback rate, this must still be implemented, and must return 1.0.
#[derive(Debug)]
pub struct Rate;
impl Property for Rate {
    type Output = f64;
    type ParseAs = f64;

    fn interface(&self) -> Interface {
        Interface::Player
    }

    fn into_output(&self, value: Self::ParseAs) -> Self::Output {
        value.into()
    }

    fn name(&self) -> &'static str {
        "Rate"
    }
}
impl ControlWritableProperty for Rate {
    fn from_output(&self, value: Self::Output) -> Self::ParseAs {
        value.into()
    }
}

pub const MINIMUMRATE: MinimumRate = MinimumRate;
/// The minimum value which the Rate property can take. Clients should not attempt to set the Rate property below this value.
/// <br>Note that even if this value is 0.0 or negative, clients should not attempt to set the Rate property to 0.0.
/// <br>This value should always be 1.0 or less, but some players might return [zbus::fdo::Error::NotSupported].
#[derive(Debug)]
pub struct MinimumRate;
impl Property for MinimumRate {
    type Output = f64;
    type ParseAs = f64;

    fn interface(&self) -> Interface {
        Interface::Player
    }

    fn into_output(&self, value: Self::ParseAs) -> Self::Output {
        value.into()
    }

    fn name(&self) -> &'static str {
        "MinimumRate"
    }
}

pub const MAXIMUMRATE: MaximumRate = MaximumRate;
/// The maximum value which the Rate property can take. Clients should not attempt to set the Rate property above this value.
/// <br>This value should always be 1.0 or greater, but some players might return [zbus::fdo::Error::NotSupported].
#[derive(Debug)]
pub struct MaximumRate;
impl Property for MaximumRate {
    type Output = f64;
    type ParseAs = f64;

    fn interface(&self) -> Interface {
        Interface::Player
    }

    fn into_output(&self, value: Self::ParseAs) -> Self::Output {
        value.into()
    }

    fn name(&self) -> &'static str {
        "MaximumRate"
    }
}

pub const POSITION: Position = Position;
/// The current track position, between 0 and the 'mpris:length' metadata entry (see [Metadata]).
/// <br>Note: If the media player allows it, the current playback position can be changed either the SetPosition method or the Seek.
/// <br>If the playback progresses in a way that is inconstistant with the Rate property, the Seeked signal is emited.
#[derive(Debug)]
pub struct Position;
impl Property for Position {
    type Output = Duration;
    type ParseAs = i64;

    fn interface(&self) -> Interface {
        Interface::Player
    }

    fn into_output(&self, value: Self::ParseAs) -> Self::Output {
        Duration::from_micros(value as u64)
    }

    fn name(&self) -> &'static str {
        "Position"
    }
}

pub const SHUFFLE: Shuffle = Shuffle;
/// A value of false indicates that playback is progressing linearly through a playlist, while true means playback is progressing through a playlist in some other order.
#[derive(Debug)]
pub struct Shuffle;
impl Property for Shuffle {
    type Output = bool;
    type ParseAs = bool;

    fn interface(&self) -> Interface {
        Interface::Player
    }

    fn into_output(&self, value: Self::ParseAs) -> Self::Output {
        value.into()
    }

    fn name(&self) -> &'static str {
        "Shuffle"
    }
}
impl ControlWritableProperty for Shuffle {
    fn from_output(&self, value: Self::Output) -> Self::ParseAs {
        value.into()
    }
}

pub const VOLUME: Volume = Volume;
/// Should be between 0.0 and 1.0, while higher settings are possible as well (but not reccommended).
#[derive(Debug)]
pub struct Volume;
impl Property for Volume {
    type Output = f64;
    type ParseAs = f64;

    fn interface(&self) -> Interface {
        Interface::Player
    }

    fn into_output(&self, value: Self::ParseAs) -> Self::Output {
        value.into()
    }

    fn name(&self) -> &'static str {
        "Volume"
    }
}
impl ControlWritableProperty for Volume {
    fn from_output(&self, value: Self::Output) -> Self::ParseAs {
        value.into()
    }
}

pub const METADATA: Metadata = Metadata;
/// See [super::Metadata] for more details
#[derive(Debug)]
pub struct Metadata;
impl Property for Metadata {
    type Output = Mtd;
    type ParseAs = HashMap<String, OwnedValue>;

    fn interface(&self) -> Interface {
        Interface::Player
    }

    fn into_output(&self, value: Self::ParseAs) -> Self::Output {
        value.into()
    }

    fn name(&self) -> &'static str {
        "Metadata"
    }
}

pub const CANGONEXT: CanGoNext = CanGoNext;
/// Whether it's possible to call [super::Player::next] method and expect the current track to change.
/// <br>(Even when playback can generally be controlled, there may not always be a next track to move to)
#[derive(Debug)]
pub struct CanGoNext;
impl Property for CanGoNext {
    type Output = bool;
    type ParseAs = bool;

    fn interface(&self) -> Interface {
        Interface::Player
    }

    fn into_output(&self, value: Self::ParseAs) -> Self::Output {
        value.into()
    }

    fn name(&self) -> &'static str {
        "CanGoNext"
    }
}

pub const CANGOPREVIOUS: CanGoPrevious = CanGoPrevious;
/// Whether the client can call the Previous method on this interface and expect the current track to change.
/// <br>Even when playback can generally be controlled, there may not always be a next previous to move to.
#[derive(Debug)]
pub struct CanGoPrevious;
impl Property for CanGoPrevious {
    type Output = bool;
    type ParseAs = bool;

    fn interface(&self) -> Interface {
        Interface::Player
    }

    fn into_output(&self, value: Self::ParseAs) -> Self::Output {
        value.into()
    }

    fn name(&self) -> &'static str {
        "CanGoPrevious"
    }
}

pub const CANPLAY: CanPlay = CanPlay;
/// Whether playback can be started using Play or PlayPause.
/// <br>Even when playback can generally be controlled, it may not be possible to enter a "playing" state, for example if there is no "current track".
#[derive(Debug)]
pub struct CanPlay;
impl Property for CanPlay {
    type Output = bool;
    type ParseAs = bool;

    fn interface(&self) -> Interface {
        Interface::Player
    }

    fn into_output(&self, value: Self::ParseAs) -> Self::Output {
        value.into()
    }

    fn name(&self) -> &'static str {
        "CanPlay"
    }
}

pub const CANPAUSE: CanPause = CanPause;
/// Whether playback can be paused using Pause or PlayPause.
/// <br>Not all media is pausable: it may not be possible to pause some streamed media, for example.
#[derive(Debug)]
pub struct CanPause;
impl Property for CanPause {
    type Output = bool;
    type ParseAs = bool;

    fn interface(&self) -> Interface {
        Interface::Player
    }

    fn into_output(&self, value: Self::ParseAs) -> Self::Output {
        value.into()
    }

    fn name(&self) -> &'static str {
        "CanPause"
    }
}

pub const CANSEEK: CanSeek = CanSeek;
/// Whether the client can control the playback position using Seek and SetPosition. This may be different for different tracks.
/// <br>If [CanControl] is false, this should be (considered) false too.
/// <br>Not all media is seekable: it may not be possible to seek when playing some streamed media, for example.
#[derive(Debug)]
pub struct CanSeek;
impl Property for CanSeek {
    type Output = bool;
    type ParseAs = bool;

    fn interface(&self) -> Interface {
        Interface::Player
    }

    fn into_output(&self, value: Self::ParseAs) -> Self::Output {
        value.into()
    }

    fn name(&self) -> &'static str {
        "CanSeek"
    }
}
