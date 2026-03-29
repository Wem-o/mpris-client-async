use std::{pin::Pin, time::Duration};

use futures::{Stream, stream::SelectAll};
use zbus::names::OwnedBusName;

use crate::{
    Mpris, Player, PlayerEvent,
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

pub struct PlayerLoop<'a> {
    // A list of tracked properties
    properties: Vec<Box<dyn AnyProperty>>,
    track_position: bool,

    // property_streams: SelectAll<ParsedPropertyStream<'a, AnyProperty>>,
    property_streams: SelectAll<Pin<Box<dyn Stream<Item = AnyStreamYield> + Send + 'a>>>,
    position_streams: SelectAll<PositionStream<'a>>,
    player_stream: SelectAll<Box<dyn Stream<Item = PlayerEvent>>>,
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
