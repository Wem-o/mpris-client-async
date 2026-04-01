use futures::StreamExt;
use mpris_client_async::{Mpris, MprisEvent, properties::*, streams::PropertyYield};

// TODO: Metadata: remove the " from the title / artists
// TODO: Add signal parsing to the event loop as well
// TODO: Implement other 2 interfaces

#[tokio::main]
async fn main() {
    // Create mpris client
    let mpris: Mpris = Mpris::new().await.unwrap();

    let mut event_loop = mpris
        .new_event_loop(vec![PlaybackStatus.into_any(), Metadata.into_any()], false)
        .await
        .expect("Failed to create event loop: {0}");

    while let Some(event) = event_loop.next().await {
        match event {
            MprisEvent::Added(player) => {
                println!("Player added with name: {}", player.dbus_name());
            }
            MprisEvent::Removed(name) => {
                println!("Player removed with name: {}", name);
            }
            MprisEvent::PositionChanged(value) => println!(
                "Player with name \"{}\" is now at {}s",
                value.player_name,
                value.value.as_secs()
            ),
            MprisEvent::PropertyChaned(prop) => {
                // prop.value is Any, so downcast them to a Property
                // that you gave in the `properties` field while creating
                // the loop

                if let Some(meta) = prop.value.downcast_ref::<PropertyYield<Metadata>>() {
                    println!(
                        "Player {} now playing track with title: {} from album {}, from artist {}",
                        meta.player_name,
                        meta.value.title,
                        meta.value.album,
                        meta.value.artists.get(0).unwrap_or(&"??".to_string())
                    );
                } else if let Some(playback) =
                    prop.value.downcast_ref::<PropertyYield<PlaybackStatus>>()
                {
                    println!("Player {} is now {}", playback.player_name, playback.value)
                }
            }
        }
    }
}
