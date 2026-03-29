use std::fmt;

/// A dbus, MPRIS interface
#[derive(Debug, Default)]
pub enum Interface {
    #[default]
    MediaPlayer2,
    Player,
    TrackList,
    Playlists,
}
impl ToString for Interface {
    fn to_string(&self) -> String {
        use Interface::*;
        match *self {
            MediaPlayer2 => String::from("org.mpris.MediaPlayer2"),
            Player => String::from("org.mpris.MediaPlayer2.Player"),
            TrackList => String::from("org.mpris.MediaPlayer2.TrackList"),
            Playlists => String::from("org.mpris.MediaPlayer2.Playlists"),
        }
    }
}
impl Interface {
    pub fn as_str(&self) -> &'static str {
        use Interface::*;
        match *self {
            MediaPlayer2 => "org.mpris.MediaPlayer2",
            Player => "org.mpris.MediaPlayer2.Player",
            TrackList => "org.mpris.MediaPlayer2.TrackList",
            Playlists => "org.mpris.MediaPlayer2.Playlists",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
/// The state of the playback
pub enum Playback {
    Playing,
    Paused,
    #[default]
    Stopped,
}
impl Playback {
    pub fn to_string(&self) -> String {
        match *self {
            Playback::Paused => "Paused",
            Playback::Playing => "Playing",
            Playback::Stopped => "Stopped",
        }
        .to_string()
    }
}
impl From<String> for Playback {
    fn from(value: String) -> Self {
        let value = value.to_lowercase();
        if value == String::from("playing") {
            Self::Playing
        } else if value == String::from("paused") {
            Self::Paused
        } else {
            Self::Stopped
        }
    }
}
impl From<&str> for Playback {
    fn from(value: &str) -> Self {
        let value = value.to_lowercase();
        if value == String::from("playing") {
            Self::Playing
        } else if value == String::from("paused") {
            Self::Paused
        } else {
            Self::Stopped
        }
    }
}
impl fmt::Display for Playback {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
/// The state of the loop
pub enum Loop {
    #[default]
    /// The playback will stop after the end of the playlist
    None,
    /// The current track will repeat forever
    Track,
    /// The whole playlist will be repeated
    Playlist,
}
impl Loop {
    pub fn to_string(&self) -> String {
        match *self {
            Loop::None => "None",
            Loop::Track => "Track",
            Loop::Playlist => "Playlist",
        }
        .to_string()
    }
}
impl From<String> for Loop {
    fn from(value: String) -> Self {
        let value = value.to_lowercase();
        if value == String::from("playlist") {
            Self::Playlist
        } else if value == String::from("track") {
            Self::Track
        } else {
            Self::None
        }
    }
}
impl From<&str> for Loop {
    fn from(value: &str) -> Self {
        let value = value.to_lowercase();
        if value == String::from("playlist") {
            Self::Playlist
        } else if value == String::from("track") {
            Self::Track
        } else {
            Self::None
        }
    }
}
impl fmt::Display for Loop {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}
