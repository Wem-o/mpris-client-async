use std::{pin::Pin, sync::Arc, time::Duration};

use async_std::sync::Mutex;

use futures::{Stream, future::join_all, stream::SelectAll};
use zbus::names::OwnedBusName;

use crate::{
    Mpris, Player,
    mpris::player_stream::PlayerStream,
    properties::{AnyProperty, AnyStreamYield},
    streams::{PositionStream, StreamYield},
};

pub enum MprisEvent<P>
where
    P: Send + 'static,
{
    Added(Player),
    Removed(OwnedBusName),
    PropertyChaned(StreamYield<P>),
    PositionChanged(StreamYield<Duration>),
}

impl<'a> Mpris<'a> {}

pub struct PlayerLoop {
    // A list of tracked properties
    properties: Vec<Box<dyn AnyProperty + Send + Sync>>,
    track_position: bool,

    players: Vec<Arc<Player>>,

    property_streams: SelectAll<Pin<Box<dyn Stream<Item = AnyStreamYield> + Send>>>,
    position_streams: SelectAll<Pin<Box<PositionStream>>>,
    player_stream: PlayerStream,
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
        })
    }
}
// impl<'a, P> Stream for PlayerLoop<'a, P> {
//     type Item = MprisEvent;

//     fn poll_next(
//         self: std::pin::Pin<&mut Self>,
//         cx: &mut std::task::Context<'_>,
//     ) -> std::task::Poll<Option<Self::Item>> {
//         use std::task::Poll::*;

//         select! {
//             property = self.property_streams => {}
//         }

//         Pending
//     }
// }
