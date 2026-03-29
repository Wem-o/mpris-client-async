use std::{pin::Pin, time::Duration};

use futures::{StreamExt, stream::select_all};
use mpris_client_async::{Mpris, properties::*, signals::Seeked, streams::PositionStream};

#[tokio::main]
async fn main() {
    // Create mpris client
    let mpris: Mpris = Mpris::new().await.unwrap();
    // Get mpris players
    let players = mpris.get_players().await.unwrap();

    // METHOD 1: Use an async event loop to listen for events
    // Note: we separate a property change from a position change because a
    // posoiton change is much more common than a property change, and you might
    // not even need it always
    //
    // In this case you keep a list of the players that you update according to the loop

    //= while let Some(event: Event) = mpris::get_event_loop(vec![PlaybackStatus, LoopStatus], track_position=true).await {
    //=     match event {
    //=         PlayerAdded(player: Player) => println!("New player added with name: {}", player.dbus_name()),
    //=         PlayerRemoved(player: Player) => println!("A player disconnected from the bus with name: {}". player.dbus_name()),
    //=         PlayerPropertyChanged(property: StreamYield) => println!("Property of {} changed to {}", property.player_name, property.value),
    //=         PlaybackPositionChanged(value: StreamYield) => println!("Player with name {} is now at position {}s", value.player_name, value.value)
    //=     }
    //= }

    let mut playback_streams = Vec::new();
    let mut seeked_signals = Vec::new();
    let mut pos_changes: Vec<Pin<Box<PositionStream>>> = vec![];

    // Get the "unique name" of the players
    for player in &players {
        println!(
            "Player: {}, with identity {} with desktop entry {}",
            player.dbus_name(),
            player.get(Identity).await.unwrap_or("??".to_string()),
            player.get(DesktopEntry).await.unwrap_or("??".to_string())
        );

        println!(
            "Player identity: {}",
            player.get(Identity).await.unwrap_or(String::from("??"))
        );

        println!("\tMediaPlayer2:");
        println!("\t\tCapabilities:");
        println!(
            "\t\t\tcan quit? {}",
            player.get(CanQuit).await.unwrap_or(false)
        );
        println!(
            "\t\t\tcan set fullscreen? {}",
            player.get(CanSetFullscreen).await.unwrap_or(false)
        );
        println!(
            "\t\t\tcan raise? {}",
            player.get(CanRaise).await.unwrap_or(false)
        );
        println!(
            "\t\t\thas track list? {}",
            player.get(HasTrackList).await.unwrap_or(false)
        );
        println!(
            "\t\t\tsupported URI: {:?}",
            player.get(SupportedURIs).await.unwrap_or(vec![])
        );
        println!(
            "\t\t\tsupported MIME types: {:?}",
            player.get(SupportedMIMEs).await.unwrap_or(vec![])
        );

        // if player.get(CanSetFullscreen).await.unwrap_or(false) {
        //     player.set(Fullscreen, !player.get(Fullscreen).await.unwrap_or(false)).await.expect("Failed to set fullscreen.");
        // }

        println!("\tMediaPlayer2.Player:");
        println!(
            "\t\tPlaybackStatus: {}",
            player
                .get(PlaybackStatus)
                .await
                .unwrap_or(mpris_client_async::Playback::Stopped)
        );
        println!(
            "\t\tLoopStatus: {}",
            player
                .get(LoopStatus)
                .await
                .unwrap_or(mpris_client_async::Loop::None)
        );
        println!("\t\trate: {}", player.get(Rate).await.unwrap_or(1.0));
        println!(
            "\t\tmax rate: {:?}, min rate: {:?}",
            player.get(MinimumRate).await,
            player.get(MaximumRate).await
        );
        println!(
            "\t\tis shuffled: {}",
            player.get(Shuffle).await.unwrap_or(false)
        );
        println!("\t\tvolume: {}", player.get(Volume).await.unwrap_or(0.0));
        println!(
            "\t\tPosition (in secs): {}",
            player
                .get(Position)
                .await
                .unwrap_or(Duration::from_secs(0))
                .as_secs()
        );
        println!(
            "\t\tcan_seek: {}",
            player.get(CanSeek).await.unwrap_or(false)
        );
        println!(
            "\t\tcan_control: {}",
            player.get(CanControl).await.unwrap_or(false)
        );

        println!("\t\t\tMetadata: {:#?}", player.get(Metadata).await);

        // let can_control = player.get(CanControl).await.unwrap_or(false);
        // if can_control {
        //     let result = player.set_controlled(LoopStatus, Loop::Track).await;
        //     println!("loop status set result: {:?}", result);
        // }
        // let min_rate = player.get(MinimumRate).await;
        // if let Ok(_) = min_rate && can_control {
        //     let result = player.set_controlled(Rate, 0.5).await;
        //     println!("set rate result: {:?}", result);
        // }
        // let path_to_media = String::from("file:///home/USER/definitely_not_hentai.mp3");
        // if can_control && player.get(SupportedURIs).await.unwrap_or(vec![]).contains(&String::from("file")) {
        //     player.open_uri(path_to_media).await.expect("Failed to open peak");
        // }

        // Subscribe to the event when Metadata changed.
        // let streams: mpris_client_async::streams::ParsedPropertyStream<'_, Rate> =
        //     player.property_changed_stream(Metadata).await.unwrap();
        playback_streams.push(
            player
                .clone()
                .subscribe_property_change(PlaybackStatus)
                .await
                .unwrap(),
        );
        seeked_signals.push(player.clone().subscribe(Seeked).await.unwrap());

        pos_changes.push(Box::pin(player.clone().subscribe_position().await.unwrap()));

        println!();
    }

    // Combine the streams of the changes. YOU CANNOT KNOW WHICH PLAYER A MESSAGE IS FROM!
    // let mut combined = select_all(playback_streams);
    // while let Some(new) = combined.next().await {
    //     println!("Playback for \"{}\" changed to: {}", new.player_name, new.value);
    // }

    // Listen for seeked signals (wont know which player it is from, as they're combined)
    // let mut combined = select_all(seeked_signals);
    // while let Some(seeked_to) = combined.next().await {
    //     println!("Player \"{}\" seeked to: {}", seeked_to.player_name, seeked_to.value.as_secs());
    // }

    // Print the changes of the media playback (estimated) position of the first player
    let mut combined = select_all(pos_changes);
    while let Some(pos) = combined.next().await {
        println!(
            "Player \"{}\" changed position to: {}",
            pos.player_name,
            pos.value.as_secs()
        )
    }

    // Listen and print if a new device connects or disconnects
    // let player_stream = mpris.player_stream().await.expect("Failed to subscribe to player_stream");
    // pin_mut!(player_stream);
    // while let Some(event) = player_stream.next().await {
    //     match event {
    //         PlayerEvent::Connected(player) => println!("New player connected with name: {}", player.dbus_name().to_string()),
    //         PlayerEvent::Disconnected(player) => println!("Player disconnected with name: {}", player.dbus_name().to_string())
    //     }
    // }
}
