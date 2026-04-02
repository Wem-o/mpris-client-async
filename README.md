# (yet another) MPRIS async client

## Why?
There are plenty of MPRIS clients out there, but I didnt like any, and i needed one for my main project.

The goal was also to abstract every low level zbus/dbus ~~bullsh*t~~ shenanigans.

## How is it different?
It's type safe. You dont specify what you want to get by just blindly passsing in strings, and hope it will work.
MPRIS properties and signals are represented by zero-sized types, implementing the Property and Signal traits respectively.

To make your life even easier, and your autocomplete just a bit happier, you cannot try to set a property that you cant.
Propeties that can be always or when the player is controllable ("Can Control is true"), implement different traits.


## And more
There is even an event loop. You can use the crate by just listening to specific stuff yourself,
or you can just use the built-in event loop. Just pass the types you want to obseve the change of, 
and the rest will be done by the magic of streams.

Dont take my word for it, take a look:
```rust
// Create mpris client
let mpris: Mpris = Mpris::new().await.unwrap();

// Get the event loop, and pass in the properties you want to observe
// you can also watch the position changes in the players, more about that later
let mut event_loop = mpris
    .new_event_loop(vec![PlaybackStatus.into_any(), Metadata.into_any()], true)
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

            // Downcast for the types. It's a bit more tedius
            // than a simple match on an enum, but not that much worse
            if let Some(meta) = prop.value.downcast_ref::<PropertyYield<Metadata>>() {
                println!(
                    "Player \"{}\" now playing track with title: \"{}\" from album \"{}\", from artist \"{}\".",
                    meta.player_name,
                    meta.value.title,
                    meta.value.album,
                    meta.value.artists.get(0).unwrap_or(&"??".to_string()),
                );
            } else if let Some(playback) =
                prop.value.downcast_ref::<PropertyYield<PlaybackStatus>>()
            {
                println!("Player {} is now {}", playback.player_name, playback.value)
            }
        }
    }
}
```

but nothing stops you from just `mpris.get_players()` and handle them manually.
If you do you can use the helper streams to get parsed signal, and property streams,
meaning you wont have to parse it yourself.
