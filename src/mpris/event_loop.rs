use std::{
    ops::Deref,
    pin::Pin,
    sync::Arc,
    task::{
        Context,
        Poll::{self, *},
    },
    time::Duration,
};

use async_std::sync::Mutex;

use futures::{Stream, StreamExt, TryFutureExt, future::join_all, stream::SelectAll};

use zbus::names::OwnedBusName;

use crate::{
    Mpris, Player, PlayerEvent,
    mpris::player_stream::PlayerStream,
    properties::{AnyProperty, AnyStreamYield},
    streams::{PositionStream, StreamYield},
};

impl<'a> Mpris<'a> {
    pub async fn new_event_loop(
        &self,
        properties: Vec<Box<dyn AnyProperty + Send + Sync>>,
        track_position: bool,
    ) -> Result<PlayerLoop, zbus::Error> {
        Ok(PlayerLoop::new(
            self.get_players().await?,
            properties,
            self.player_stream().await?,
            track_position,
        )
        .await?)
    }
}

#[derive(Debug)]
pub enum MprisEvent {
    Added(Arc<Player>),
    Removed(OwnedBusName),
    PropertyChaned(AnyStreamYield),
    PositionChanged(StreamYield<Duration>),
}
impl From<PlayerEvent> for MprisEvent {
    fn from(value: PlayerEvent) -> Self {
        match value {
            PlayerEvent::Connected(player) => Self::Added(player),
            PlayerEvent::Disconnected(player) => Self::Removed(player.dbus_name()),
        }
    }
}
impl From<&PlayerEvent> for MprisEvent {
    fn from(value: &PlayerEvent) -> Self {
        match value {
            PlayerEvent::Connected(player) => Self::Added(player.clone()),
            PlayerEvent::Disconnected(player) => Self::Removed(player.dbus_name()),
        }
    }
}
impl From<&StreamYield<Duration>> for MprisEvent {
    fn from(value: &StreamYield<Duration>) -> Self {
        Self::PositionChanged(value.clone())
    }
}
impl From<&AnyStreamYield> for MprisEvent {
    fn from(value: &AnyStreamYield) -> Self {
        // Self::PropertyChaned(value.deref().clone())
        unimplemented!()
    }
}

pub struct PlayerLoop {
    // A list of tracked properties
    properties: Vec<Box<dyn AnyProperty + Send + Sync>>,
    track_position: bool,

    players: Vec<Arc<Player>>,

    property_streams: SelectAll<Pin<Box<dyn Stream<Item = AnyStreamYield> + Send>>>,
    position_streams: SelectAll<Pin<Box<PositionStream>>>,
    player_stream: PlayerStream,

    pending_get_position_streams: Option<
        Pin<
            Box<
                dyn Future<Output = Result<SelectAll<Pin<Box<PositionStream>>>, zbus::Error>>
                    + Send
                    + 'static,
            >,
        >,
    >,

    pending_get_property_streams: Option<
        Pin<
            Box<
                dyn Future<
                        Output = Result<
                            SelectAll<Pin<Box<dyn Stream<Item = AnyStreamYield> + Send>>>,
                            zbus::Error,
                        >,
                    > + Send
                    + 'static,
            >,
        >,
    >,
}
impl PlayerLoop {
    async fn get_property_streams(
        players: Vec<Arc<Player>>,
        properties: Vec<Box<dyn AnyProperty + Send + Sync + 'static>>,
    ) -> Result<SelectAll<Pin<Box<dyn Stream<Item = AnyStreamYield> + Send>>>, zbus::Error> {
        let streams = Arc::new(Mutex::new(SelectAll::new()));

        // To get the streams for all tracked propeties we need to
        // First iter through the properties
        let streams_clone = streams.clone();
        join_all(properties.into_iter().map(move |property| {
            let players = players.clone();
            let streams = streams_clone.clone();
            tokio::spawn(async move {
                // First push elements of a player into this, so there
                // is no need to lock every time for the main streams
                let mut property_streams = SelectAll::new();

                // Second iter through the players and get
                // the stream of a property from each of them
                join_all(
                    players
                        .iter()
                        .map(|player| property.subscribe_erased(Arc::clone(player))),
                )
                .await // Wait till all the stream are ready
                .into_iter() // Consume them
                .try_fold(Vec::new(), |mut filtered, maybe_stream| {
                    filtered.push(maybe_stream?);
                    Ok::<
                        Vec<Pin<Box<dyn Stream<Item = AnyStreamYield> + std::marker::Send>>>,
                        zbus::Error,
                    >(filtered)
                })? // Get the inner values from them, and if any is an Err abort the function
                .into_iter() // Consume them again
                .for_each(|stream| property_streams.push(stream)); // Append them to the property stream

                streams.lock().await.extend(property_streams);

                Ok::<(), zbus::Error>(()) // Needed so the async block returns a Result, and so we can use the ?
            })
        }))
        .await;

        let lock = match Arc::try_unwrap(streams) {
            Ok(v) => v,
            Err(_) => {
                unreachable!("Not all pointers were cleared in `get_property_streams`")
            }
        };
        Ok(lock.into_inner())
    }

    async fn get_position_streams(
        players: Vec<Arc<Player>>,
    ) -> Result<SelectAll<Pin<Box<PositionStream>>>, zbus::Error> {
        let mut streams = SelectAll::new();

        join_all(
            players
                .into_iter()
                .map(|player| player.subscribe_position()),
        )
        .await
        .into_iter()
        .try_fold(Vec::new(), |mut folded, pos_stream| {
            folded.push(pos_stream?);
            Ok::<Vec<PositionStream>, zbus::Error>(folded)
        })?
        .into_iter()
        .for_each(|pos_stream| streams.push(Box::pin(pos_stream)));

        Ok(streams)
    }

    pub async fn new(
        players: Vec<Arc<Player>>,
        properties: Vec<Box<dyn AnyProperty + Send + Sync>>,
        player_stream: PlayerStream,
        track_position: bool,
    ) -> Result<Self, zbus::Error> {
        Ok(Self {
            property_streams: Self::get_property_streams(
                players.clone(),
                properties.iter().map(|this| this.clone_box()).collect(),
            )
            .await?,
            position_streams: if track_position {
                Self::get_position_streams(players.clone()).await?
            } else {
                SelectAll::new()
            },
            player_stream,

            players,
            track_position,
            properties,

            pending_get_position_streams: None,
            pending_get_property_streams: None,
        })
    }

    /// Hanldes a pending async task
    ///
    /// Returns Pending if the future is not yet ready,
    /// Ready(None) if future returned with error,
    /// Ready(Some()) if the value was set successfully
    fn handle_pending<T, E>(
        pender: &mut Pin<Box<dyn Future<Output = Result<T, E>> + Send + 'static>>,
        set_on_success: &mut T,
        cx: &mut Context<'_>,
    ) -> Poll<Option<()>> {
        match pender.try_poll_unpin(cx) {
            Pending => Pending,
            Ready(Err(_e)) => Ready(None),
            Ready(Ok(v)) => {
                *set_on_success = v;
                Ready(Some(()))
            }
        }
    }
}
impl Stream for PlayerLoop {
    type Item = MprisEvent;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // You should follow the numbers (startind with 1.1) to read this

        let this = self.get_mut();

        // TODO: Yield the current state

        // 1.2 Check the position_streams
        if let Some(position_streams) = this.pending_get_position_streams.as_mut() {
            match Self::handle_pending(position_streams, &mut this.position_streams, cx) {
                Pending => return Pending,
                Ready(None) => return Ready(None),
                Ready(Some(())) => this.pending_get_position_streams = None,
            }
        }

        // 1.2 Check the property_streams
        if let Some(prop_streams) = this.pending_get_property_streams.as_mut() {
            match Self::handle_pending(prop_streams, &mut this.property_streams, cx) {
                Pending => return Pending,
                Ready(None) => return Ready(None),
                Ready(Some(())) => this.pending_get_property_streams = None,
            }
        }

        // 1.1 Poll players
        match &this.player_stream.poll_next_unpin(cx) {
            Pending => {}
            Ready(None) => return Ready(None),
            Ready(Some(event)) => {
                // Update list of players
                match &event {
                    PlayerEvent::Connected(player) => {
                        this.players.push(player.clone());
                    }
                    PlayerEvent::Disconnected(player) => {
                        this.players.retain(|other| *player != *other);
                    }
                };

                // Update internal streams
                // Update position stream, 1.2 when the future finishes
                if this.track_position {
                    let player_clone = this.players.clone();
                    this.pending_get_position_streams = Some(Box::pin(async move {
                        Self::get_position_streams(player_clone).await
                    }));

                    // Poll future, so even if its not ready
                    // the stream will be run again when it is
                    // by the context
                    match Self::handle_pending(
                        this.pending_get_position_streams.as_mut().unwrap(),
                        &mut this.position_streams,
                        cx,
                    ) {
                        Ready(None) => return Ready(None),
                        Ready(Some(())) => this.pending_get_position_streams = None,
                        _ => {}
                    }
                }

                // Update property streams, 1.3 when future finishes
                let player_clone = this.players.clone();
                let prop_clone = this
                    .properties
                    .iter()
                    .map(|this| this.clone_box())
                    .collect();
                this.pending_get_property_streams = Some(Box::pin(async move {
                    Self::get_property_streams(player_clone, prop_clone).await
                }));
                // Poll future, so even if its not ready
                // the stream will be run again when it is
                // by the context
                match Self::handle_pending(
                    this.pending_get_property_streams.as_mut().unwrap(),
                    &mut this.property_streams,
                    cx,
                ) {
                    Ready(None) => return Ready(None),
                    Ready(Some(())) => this.pending_get_property_streams = None,
                    _ => {}
                }

                return Ready(Some(event.into()));
            }
        }

        // 2 Poll current position
        match &this.position_streams.poll_next_unpin(cx) {
            Pending => Pending
            Ready(None) => return Ready(None),
            Ready(Some(event)) => return Ready(Some(event.into())),
        }

        // 3 Poll the tracked properties
        // match &this.property_streams.poll_next_unpin(cx) {
        //     Pending => Pending,
        //     Ready(None) => Ready(None),
        //     Ready(Some(prop)) => Ready(Some(prop.into())),
        // }
    }
}
