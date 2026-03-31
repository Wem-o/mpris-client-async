//! Provides a few useful streams to make working with a [`Player`](super::Player) easier

use std::{
    ops::Deref,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use futures::Stream;
use pin_project::pin_project;
use serde::de::DeserializeOwned;
use tokio::time::{Instant, Sleep, sleep_until};
use zbus::{
    names::OwnedBusName,
    proxy::{PropertyStream, SignalStream},
    zvariant::{OwnedValue, Type},
};

use crate::{
    Playback,
    player::Property,
    properties::{PlaybackStatus, Rate},
    signals::{Seeked, Signal},
};

#[derive(Debug, Clone)]
/// Contains the value yielded by a stream, and the [`Player`](super::Player)'s name that yielded it.
///
/// Useful when combining streams
pub struct StreamYield<T> {
    pub player_name: OwnedBusName,
    pub value: T,
}
impl<T> StreamYield<T> {
    pub(crate) fn new(player_name: OwnedBusName, value: T) -> Self {
        Self { player_name, value }
    }
}

/// Returns the current position of the media of a [`Player`](super::Player) every second, without polling the player.
///
/// Note: this doesn't take into account the length of the media, as it might not be provided, thus the returned position could be longer than the length of the media.
#[pin_project]
pub struct PositionStream {
    #[pin]
    playback_stream: ParsedPropertyStream<PlaybackStatus>,

    #[pin]
    rate_stream: ParsedPropertyStream<Rate>,

    #[pin]
    seeked_stream: ParsedSignalStream<Seeked>,

    #[pin]
    sleep: Sleep,
    // Track the last time the stream to avoid drift off the actual time (as sleep may not wake after EXACTLY 1 second)
    last_tick: Instant,

    rate: f64,
    playback: Playback,
    position: Duration,

    player_name: OwnedBusName,
}
impl PositionStream {
    pub fn new(
        player_name: OwnedBusName,
        playback_stream: ParsedPropertyStream<PlaybackStatus>,
        initial_playback: Playback,
        rate_stream: ParsedPropertyStream<Rate>,
        initial_rate: f64,
        seeked_stream: ParsedSignalStream<Seeked>,
        initial_position: Duration,
    ) -> Self {
        Self {
            playback_stream,
            rate_stream,
            seeked_stream,
            sleep: Sleep::from(sleep_until(Instant::now())), // The stream be called instantly when the first poll happens
            last_tick: Instant::now(),
            rate: initial_rate,
            playback: initial_playback,
            position: initial_position,
            player_name,
        }
    }
}
impl Stream for PositionStream {
    type Item = StreamYield<Duration>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        use Poll::*;
        let mut this = self.project();

        // Check if the rate changed
        match this.rate_stream.as_mut().poll_next(cx) {
            // Nothing changed
            Pending => {}
            Ready(None) => return Ready(None),
            Ready(Some(new_rate)) => {
                let old_rate = *this.rate;
                *this.rate = new_rate.value;

                if *this.playback == Playback::Playing {
                    // How much time passsed since the last tick
                    let delta = Instant::now() - *this.last_tick;
                    let new_position = Duration::from_micros(
                        (this.position.as_micros() as f64 + (delta.as_micros() as f64 * old_rate))
                            as u64,
                    );

                    this.sleep
                        .set(sleep_until(Instant::now() + Duration::from_secs(1)));

                    *this.last_tick = Instant::now();
                    *this.position = new_position;

                    return Ready(Some(StreamYield::new(
                        this.player_name.clone(),
                        new_position,
                    )));
                }
            }
        }

        // See if the playback status changed
        match this.playback_stream.as_mut().poll_next(cx) {
            // Playback state did not change
            Pending => {}
            // playback_stream finished, meaning this stream should finish too
            Ready(None) => return Ready(None),
            Ready(Some(new_playback)) => {
                let old_playback = *this.playback;
                *this.playback = new_playback.value;

                this.sleep
                    .set(sleep_until(Instant::now() + Duration::from_secs(1)));

                match (old_playback, *this.playback) {
                    (Playback::Paused | Playback::Stopped, Playback::Playing) => {
                        *this.last_tick = Instant::now();
                        return Ready(Some(StreamYield::new(
                            this.player_name.clone(),
                            *this.position,
                        )));
                    }
                    (Playback::Playing, Playback::Paused | Playback::Stopped) => {
                        let delta = Instant::now() - *this.last_tick;
                        *this.position = Duration::from_micros(
                            ((*this.position + delta).as_micros() as f64 * *this.rate) as u64,
                        );
                        *this.last_tick = Instant::now();

                        return Ready(Some(StreamYield::new(
                            this.player_name.clone(),
                            *this.position,
                        )));
                    }
                    _ => {}
                }
            }
        };

        match this.seeked_stream.as_mut().poll_next(cx) {
            Pending => {}
            Ready(None) => return Ready(None),
            Ready(Some(new)) => {
                *this.position = new.value;
                *this.last_tick = Instant::now();

                // Set next sleep cycle
                this.sleep
                    .set(sleep_until(Instant::now() + Duration::from_secs(1)));

                return Ready(Some(StreamYield::new(this.player_name.clone(), new.value)));
            }
        }

        match this.sleep.as_mut().poll(cx) {
            Pending => Pending,
            Ready(_) => {
                match *this.playback {
                    Playback::Playing => {
                        let delta = Instant::now() - *this.last_tick;
                        let new_position = Duration::from_micros(
                            (this.position.as_micros() as f64
                                + (delta.as_micros() as f64 * *this.rate))
                                as u64,
                        );

                        *this.position = new_position;
                    }
                    Playback::Stopped => *this.position = Duration::from_secs(0),
                    _ => {}
                }

                *this.last_tick = Instant::now();

                this.sleep
                    .set(sleep_until(Instant::now() + Duration::from_secs(1)));

                Ready(Some(StreamYield::new(
                    this.player_name.clone(),
                    *this.position,
                )))
            }
        }
    }
}

#[pin_project]
/// A [`PropertyStream`](https://docs.rs/zbus/latest/zbus/proxy/struct.PropertyStream.html), but the raw data is parsed into the corresponding [`Property`](super::properties::Property) type.
/// <br>Note: The first time the stream is polled it will return the <b>current</b> state.
///
/// <br>For signals check out [`ParsedSignalStream`]
pub struct ParsedPropertyStream<P>
where
    P: Property + Unpin + Send + Sync + 'static,
    P::ParseAs: TryFrom<OwnedValue> + Send,
    P::Output: Send,
{
    #[pin]
    raw_stream: PropertyStream<'static, P>,
    #[pin]
    pending:
        Option<Pin<Box<dyn Future<Output = Result<P::ParseAs, zbus::Error>> + 'static + Send>>>,

    p: P,
    player_name: OwnedBusName,
}
impl<P> ParsedPropertyStream<P>
where
    P: Property + Unpin + Send + Sync + 'static,
    P::ParseAs: TryFrom<OwnedValue> + Send,
    P::Output: Send + 'static,
{
    pub fn new(
        property: P,
        player_name: OwnedBusName,
        prop_stream: PropertyStream<'static, P>,
    ) -> Self {
        Self {
            raw_stream: prop_stream,
            pending: None,
            p: property,
            player_name,
        }
    }
}
impl<P> Stream for ParsedPropertyStream<P>
where
    P: Property + Unpin + Send + 'static + Sync,
    P::ParseAs: TryFrom<OwnedValue> + Send,
    P::Output: Send + 'static,
{
    type Item = StreamYield<P::Output>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        use Poll::*;
        let mut this = self.project();

        loop {
            if let Some(fut) = this.pending.as_mut().as_pin_mut() {
                match fut.poll(cx) {
                    Pending => return Pending,
                    Ready(Ok(result)) => {
                        let parsed: P::Output = this.p.into_output(result);
                        this.pending.set(None);

                        return Ready(Some(StreamYield::new(this.player_name.clone(), parsed)));
                    }
                    Ready(Err(_e)) => {
                        this.pending.set(None);
                        return Ready(None);
                    }
                }
            }

            // Try to poll the raw stream, if something is changed, start polling what changed.
            match this.raw_stream.as_mut().poll_next(cx) {
                Pending => return Pending,
                Ready(None) => return Ready(None), // The raw stream is finished, meaning this stream should finish too
                Ready(Some(value)) => {
                    // If something has changed, create a future that can be polled, to get what changed, and return Pending
                    let fut: Pin<Box<dyn Future<Output = Result<P::ParseAs, zbus::Error>> + Send>> =
                        Box::pin(async move {
                            // It is safe to unwrap, as it could only fail on UNIX platforms, if Value::Fd is being parsed
                            let value: OwnedValue = value
                                .get_raw()
                                .await?
                                .deref()
                                .clone()
                                .try_into_owned()
                                .unwrap();
                            let converted: P::ParseAs = value.try_into().map_err(|_e| {
                                zbus::Error::Variant(zbus::zvariant::Error::IncorrectType)
                            })?;
                            Ok(converted)
                        });
                    *this.pending = Some(fut);
                }
            }
        }
    }
}

#[pin_project]
/// A [`SignalStream`](https://docs.rs/zbus/latest/zbus/proxy/struct.SignalStream.html), but the raw data is parsed into the corresponding [`Signal`](super::signals::Signal) type.
///
/// <br>For properties check out [`ParsedPropertyStream`]
pub struct ParsedSignalStream<S>
where
    S: Signal + 'static,
    S::ParseAs: DeserializeOwned + Send + 'static,
{
    #[pin]
    raw_stream: SignalStream<'static>,

    s: S,
    player_name: OwnedBusName,
}
impl<S> ParsedSignalStream<S>
where
    S: Signal + 'static,
    S::ParseAs: DeserializeOwned + Send + 'static,
{
    pub fn new(signal: S, player_name: OwnedBusName, signal_stream: SignalStream<'static>) -> Self {
        Self {
            raw_stream: signal_stream,
            s: signal,
            player_name,
        }
    }
}
impl<S> Stream for ParsedSignalStream<S>
where
    S: Signal + 'static,
    S::Output: Send + 'static,
    S::ParseAs: DeserializeOwned + Send + 'static + Type,
{
    type Item = StreamYield<S::Output>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        use Poll::*;
        let mut this = self.project();

        // Try to poll the raw stream, if something is changed, start polling what changed.
        match this.raw_stream.as_mut().poll_next(cx) {
            Pending => Pending,
            Ready(None) => Ready(None), // The raw stream is finished, meaning this stream should finish too
            Ready(Some(msg)) => {
                let body = msg.body();
                let parsed: S::ParseAs = match body.deserialize_unchecked() {
                    // Lets hope unchecked is fine
                    Ok(v) => v,
                    Err(_e) => return Ready(None),
                };

                Ready(Some(StreamYield::new(
                    this.player_name.clone(),
                    this.s.into_output(parsed),
                )))
            }
        }
    }
}
