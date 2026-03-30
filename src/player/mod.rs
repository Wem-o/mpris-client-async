use futures::StreamExt;
use std::{pin::Pin, sync::Arc, time::Duration};

use futures::Stream;
use zbus::{
    Connection, Proxy, fdo,
    names::OwnedBusName,
    proxy,
    zvariant::{OwnedValue, Value},
};

mod metadata;
pub use metadata::Metadata;

pub use crate::player::properties::{ControlWritableProperty, Property, WritableProperty};
use crate::{
    player::{signals::Signal, streams::ParsedPropertyStream},
    properties::{AnyStreamYield, PlaybackStatus, Position, Rate},
    signals::Seeked,
    streams::{ParsedSignalStream, PositionStream},
};

pub mod properties;
pub mod signals;

mod enums;
pub use enums::*;

pub mod streams;

/// A player that plays something, or not, who knowns...
#[derive(Debug, Clone)]
pub struct Player {
    /// Well known name
    name: OwnedBusName,
    /// Connection to the bus
    connection: Connection,
    /// A proxy to "org.mpris.MediaPlayer2"
    proxy: Option<Proxy<'static>>,
    /// A proxy to "org.mpris.MediaPlayer2.Player"
    player_proxy: Option<Proxy<'static>>,
    /// A proxy to "org.mpris.MediaPlayer2.TrackList"
    tracklist_proxy: Option<Proxy<'static>>,
    /// A proxy to "org.mpris.MediaPlayer2.Playlists"
    playlists_proxy: Option<Proxy<'static>>,
}
impl PartialEq for Player {
    // Two players are the same, if their dbus unique names are the same
    fn eq(&self, other: &Self) -> bool {
        self.dbus_name() == other.dbus_name()
    }
}
impl Player {
    /// Create a proxy from a given name
    async fn create_proxy(
        connection: &Connection,
        name: &OwnedBusName,
        iface: Interface,
    ) -> Result<Proxy<'static>, zbus::Error> {
        Ok(proxy::Builder::new(connection)
            .destination(name.to_owned())?
            .path("/org/mpris/MediaPlayer2")?
            .interface(iface.to_string())?
            .cache_properties(proxy::CacheProperties::Yes)
            .build()
            .await?)
    }

    /// Creates an instance from a "well known name", and a connection
    pub async fn new(name: OwnedBusName, connection: Connection) -> Result<Self, zbus::Error> {
        let proxy = Self::create_proxy(&connection, &name, Interface::MediaPlayer2)
            .await
            .ok();

        let player_proxy = Self::create_proxy(&connection, &name, Interface::Player)
            .await
            .ok();

        let tracklist_proxy = Self::create_proxy(&connection, &name, Interface::TrackList)
            .await
            .ok();

        let playlists_proxy = Self::create_proxy(&connection, &name, Interface::Playlists)
            .await
            .ok();

        Ok(Self {
            name,
            connection,
            proxy,
            player_proxy,
            tracklist_proxy,
            playlists_proxy,
        })
    }

    /// Returns the ["unique name"](https://z-galaxy.github.io/zbus/concepts.html#bus-name--service-name) of the player.
    ///
    /// For example `org.mpris.MediaPlayer2.vlc`.
    pub fn dbus_name(&self) -> OwnedBusName {
        self.name.clone()
    }

    /// Returns a reference to a Proxy object from an [`Interface`]
    fn iface_to_proxy(&self, interface: Interface) -> Result<&Proxy<'static>, zbus::Error> {
        let iface = match interface {
            Interface::MediaPlayer2 => &self.proxy,
            Interface::Player => &self.player_proxy,
            Interface::Playlists => &self.playlists_proxy,
            Interface::TrackList => &self.tracklist_proxy,
        };

        match iface {
            Some(v) => Ok(&v),
            None => Err(zbus::Error::InterfaceNotFound),
        }
    }

    /// Parses a property from the player. See [`properties`] for more
    pub async fn get<P>(&self, property: P) -> Result<P::Output, zbus::Error>
    where
        P: Property,
        P::ParseAs: TryFrom<OwnedValue>,
    {
        let proxy = self.iface_to_proxy(property.interface())?;

        let value: OwnedValue = proxy.get_property(property.name()).await?;

        // Create the intermediate type
        let parsed: P::ParseAs = value
            .try_into()
            .map_err(|_e| zbus::Error::Variant(zbus::zvariant::Error::IncorrectType))?;

        Ok(property.into_output(parsed))
    }

    /// Set a property that implements [`WritableProperty`].
    ///
    /// Note: only properties that doesnt require [`properties::CanControl`] to be true
    /// can be set here. For properties that do require it see [`set_controlled`](Self::set_controlled).
    pub async fn set<'a, P>(&self, property: P, new_value: P::Output) -> Result<(), fdo::Error>
    where
        P: WritableProperty,
        P::ParseAs: 'a + Into<Value<'a>>,
    {
        let proxy = self.iface_to_proxy(property.interface())?;
        let transformed_value: P::ParseAs = property.from_output(new_value);

        proxy
            .set_property(property.name(), transformed_value)
            .await
            .map(|_| ())
    }

    /// Sets a property that requires the player to allow controlling, thus [`properties::CanControl`] must be true.
    ///
    /// Make sure to check it before using it. If not checked before use it will return with an error
    // TODO: Find the type of error a player without cancontrol can return if trying to be set.
    pub async fn set_controlled<'a, P>(
        &self,
        property: P,
        new_value: P::Output,
    ) -> Result<(), fdo::Error>
    where
        P: ControlWritableProperty,
        P::ParseAs: 'a + Into<Value<'a>>,
    {
        let proxy = self.iface_to_proxy(property.interface())?;
        let transformed_value: P::ParseAs = property.from_output(new_value);

        proxy
            .set_property(property.name(), transformed_value)
            .await
            .map(|_| ())
    }

    /// Returns a stream that fires every time a [`Property`] of some kind had been changed.
    pub async fn subscribe_property_change<P>(
        self: Arc<Self>,
        property: P,
    ) -> Result<ParsedPropertyStream<P>, zbus::Error>
    where
        P: Property + Send + Unpin + Sync + 'static,
        P::ParseAs: TryFrom<OwnedValue> + Send,
        P::Output: Send,
    {
        let proxy = self.iface_to_proxy(property.interface())?;
        let raw = proxy.receive_property_changed(property.name()).await;
        Ok(ParsedPropertyStream::new(property, self.dbus_name(), raw))
    }

    pub async fn subscribe_property_change_erased<P>(
        self: Arc<Self>,
        property: P,
    ) -> Result<Pin<Box<dyn Stream<Item = AnyStreamYield> + Send>>, zbus::Error>
    where
        P: Property + Unpin + Send + Sync + 'static,
        P::ParseAs: TryFrom<OwnedValue> + Send,
        P::Output: Send + 'static,
    {
        Ok(Box::pin(
            self.subscribe_property_change(property)
                .await?
                .map(|output| AnyStreamYield {
                    value: Box::new(output),
                }),
        ))
    }

    /// Subscribe to a D-Bus signal. Possible options: [`signals`]
    pub async fn subscribe<S>(
        self: Arc<Self>,
        signal: S,
    ) -> Result<ParsedSignalStream<S>, zbus::Error>
    where
        S: Signal + Unpin + 'static,
        S::ParseAs: TryFrom<OwnedValue>,
    {
        let proxy = self.iface_to_proxy(signal.interface())?;
        let raw = proxy.receive_signal(signal.name()).await?;

        Ok(ParsedSignalStream::new(signal, self.dbus_name(), raw))
    }

    /// Returns a [`PositionStream`] that yields the current (esitmated) position of the media playback.
    /// It does this by listening to the [`Seeked`] [`signal`](Signal) and the [`PlaybackStatus`] and [`Rate`] [`properties`](Property), and those's changes
    /// to determine the position of the playback.
    ///
    /// <br>This SHOULD be prefered over repetitively calling [`get`](Self::get), as this tracks the
    /// duration internally, instead of parsing from the bus every time.
    pub async fn subscribe_position(self: Arc<Self>) -> Result<PositionStream, zbus::Error> {
        Ok(PositionStream::new(
            self.dbus_name(),
            self.clone()
                .subscribe_property_change(PlaybackStatus)
                .await?,
            self.get(PlaybackStatus).await?,
            self.clone().subscribe_property_change(Rate).await?,
            self.get(Rate).await?,
            self.clone().subscribe(Seeked).await?,
            self.get(Position).await?,
        ))
    }

    //                             ====================
    //                             ===    METHODS   ===
    //                             ====================

    async fn call_method<A, R>(
        &self,
        method_name: &str,
        arguments: A,
        iface: Interface,
    ) -> Result<R, zbus::Error>
    where
        A: serde::Serialize + zbus::zvariant::DynamicType,
        R: for<'d> zbus::zvariant::DynamicDeserialize<'d>,
    {
        let proxy = Proxy::new(
            &self.connection,
            self.name.to_owned(),
            "/org/mpris/MediaPlayer2",
            iface.as_str(),
        )
        .await?;

        proxy.call(method_name, &arguments).await
    }

    /// Skips to the next track in the tracklist. If there is no next track (and endless playback and track repeat are both off), stop playback.
    /// <br>If playback is paused or stopped, it remains that way.
    /// <br>If [`properties::CanGoNext`] is false, attempting to call this method should have no effect.
    pub async fn next(&self) -> Result<(), zbus::Error> {
        self.call_method("Next", [()], Interface::Player).await
    }

    /// Skips to the previous track in the tracklist. If there is no previous track (and endless playback and track repeat are both off), stop playback.
    /// <br>If playback is paused or stopped, it remains that way.
    /// <br>If [`properties::CanGoPrevious`] is false, attempting to call this method should have no effect.
    pub async fn previous(&self) -> Result<(), zbus::Error> {
        self.call_method("Previous", [()], Interface::Player).await
    }

    /// Pauses the playback.
    /// If [`properties::CanPause`] is false, this should have no effect.
    pub async fn pause(&self) -> Result<(), zbus::Error> {
        self.call_method("Pause", [()], Interface::Player).await
    }

    /// Starts or resumes the playback.
    /// <br>If playback is already running or if [`properties::CanPlay`] is false, this should have no effect.
    pub async fn play(&self) -> Result<(), zbus::Error> {
        self.call_method("Play", [()], Interface::Player).await
    }

    /// Toggles the playback status between play and pause.
    /// <br>If [`properties::CanPause`] is false, this should have no effect, and may return with an error.
    pub async fn play_pause(&self) -> Result<(), zbus::Error> {
        self.call_method("PlayPause", [()], Interface::Player).await
    }

    /// Stops playback. Calling [`Self::play`] after this should restart the playlist.
    /// <br>If [`properties::CanControl`] is false, calling this should have no effect, and may raise an error.
    pub async fn stop(&self) -> Result<(), zbus::Error> {
        self.call_method("Stop", [()], Interface::Player).await
    }

    /// A duration to seek forward, or of backwards is true backwards.
    /// <br>May only be used if [`properties::CanSeek`] is true.
    pub async fn seek(&self, duration: Duration, backwards: bool) -> Result<(), zbus::Error> {
        let modified_time = duration.as_micros() as f64 * { if backwards { -1.0 } else { 1.0 } };
        self.call_method("Seek", [modified_time], Interface::Player)
            .await
    }

    /// Sets the position of the track between 0 and the [length of the track](metadata::Metadata::length). track_id can be retreived from the [metadata](metadata::Metadata::trackid), but it may <b>NOT</b> be "/org/mpris/MediaPlayer2/TrackList/NoTrack".
    /// <br>If position is greater than the [length of the track](metadata::Metadata::length), this shouldn't do anything.
    /// <br>If [properties::CanSeek] is false this should have no effect.
    pub async fn set_position(
        &self,
        track_id: String,
        position: Duration,
    ) -> Result<(), zbus::Error> {
        self.call_method(
            "SetPosition",
            [track_id, position.as_micros().to_string()],
            Interface::Player,
        )
        .await
    }

    /// Opens a URI, which's scheme should be an element of [`properties::SupportedURIs`] and the mime-type should match one of the elements of [properties::SupportedMIMEs].
    /// If not supported it should raise an error.
    /// <br>If the playback is stopped, it should be started. It also shouldnt be assumed the player opens the URI as soon as called!
    pub async fn open_uri(&self, uri: String) -> Result<(), zbus::Error> {
        self.call_method("OpenUri", [uri], Interface::Player).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn playback_status_conversion() {
        assert_eq!(Playback::Playing, Playback::from("Playing"));
        assert_eq!(Playback::Paused, Playback::from("Paused"));
        assert_eq!(Playback::Stopped, Playback::from("Stopped"));
    }

    #[test]
    fn loop_status_conversion() {
        assert_eq!(Loop::Playlist, Loop::from("Playlist"));
        assert_eq!(Loop::None, Loop::from("None"));
        assert_eq!(Loop::Track, Loop::from("Track"));
    }
}
