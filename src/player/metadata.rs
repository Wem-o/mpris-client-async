use std::{collections::HashMap, time::Duration};

use zbus::zvariant::OwnedValue;

/// Metadata of a media
/// <br>It's construced from the [metadata specs](https://www.freedesktop.org/wiki/Specifications/mpris-spec/metadata/).
/// <br>Dont assume any of this is actually provided (other than trackid), but basics such as title, artists, and sometimes the album is provided.
#[derive(Debug, Clone)]
pub struct Metadata {
    // MPRIS specific things
    /// A unique identity for this track within the context of an MPRIS object.
    /// <br>This is always provided, but not always accurate (for example browsers might provide bullshit).
    pub trackid: String,
    /// The length of the track
    pub length: Option<Duration>,
    /// The URI of the location of the track. You should not assume this will exist when a new track is played.
    /// <br>Local files will start "file://", but it can be an online URL as well (for example Spotify's desktop player provides a URL).
    pub art_url: Option<String>,

    // XESAM fields
    /// The album name
    pub album: String,
    /// A list of artists
    pub artists: Vec<String>,
    /// The list of the album's artists
    pub album_artist: Vec<String>,
    /// The title of the media
    pub title: String,

    /// The lyrics. (Corresponds to "xesam:asText")
    pub lyrics: String,
    /// BPM of the song
    pub bpm: i64,
    /// An automatically-generated rating, based on things such as how often it has been played. This should be in between 0.0 and 1.0.
    pub auto_rating: f64,
    /// A rating given to the track by the user, between 0.0 and 1.0
    pub user_rating: f64,
    /// A list of comments
    pub comments: Vec<String>,
    /// The composers of the song
    pub composers: Vec<String>,
    /// The disc number on the album that the track is from
    pub disc_number: i64,
    /// The track number on the album disc
    pub track_number: i64,
    /// The location of the file
    pub url: String,

    /// The genres of the media
    pub genres: Vec<String>,
    /// The lyricists of the media
    pub lyricists: Vec<String>,

    /// The time when the media was created. Should follow ISO 8601.
    /// <br>xesam:contentCreated
    pub created: String,
    /// The time when the was first played. Should follow ISO 8601.
    pub first_used: String,
    /// The time when the was last played. Should follow ISO 8601.
    pub last_used: String,
    /// The number of times the track has been played
    pub use_count: i64,
}
impl Metadata {
    pub fn new_from_hashmap(map: HashMap<String, OwnedValue>) -> Self {
        Self {
            trackid: match map.get("mpris:trackid") {
                Some(id) => id.to_string(),
                None => String::new(),
            },
            length: map.get("mpris:length").map_or(None, |value| {
                value
                    .downcast_ref::<i64>()
                    .ok()
                    .map(|d| Duration::from_micros(d as u64))
            }),
            art_url: map
                .get("mpris:artUrl")
                .map_or(None, |value| Some(value.to_string())),

            album: map
                .get("xesam:album")
                .map_or(String::new(), |value| value.to_string()),
            album_artist: map.get("xesam:albumArtists").map_or(Vec::new(), |value| {
                Vec::<String>::try_from(value.clone()).map_or(Vec::new(), |v| v)
            }),
            artists: map.get("xesam:artist").map_or(Vec::new(), |value| {
                Vec::<String>::try_from(value.clone()).map_or(Vec::new(), |v| v)
            }),
            comments: map.get("xesam:comment").map_or(Vec::new(), |value| {
                Vec::<String>::try_from(value.clone()).map_or(Vec::new(), |v| v)
            }),
            lyricists: map.get("xesam:lyricist").map_or(Vec::new(), |value| {
                Vec::<String>::try_from(value.clone()).map_or(Vec::new(), |v| v)
            }),
            composers: map.get("xesam:composer").map_or(Vec::new(), |value| {
                Vec::<String>::try_from(value.clone()).map_or(Vec::new(), |v| v)
            }),
            genres: map.get("xesam:genre").map_or(Vec::new(), |value| {
                Vec::<String>::try_from(value.clone()).map_or(Vec::new(), |v| v)
            }),

            lyrics: map
                .get("mpris:asText")
                .map_or(String::new(), |value| value.to_string()),
            url: map
                .get("mpris:url")
                .map_or(String::new(), |value| value.to_string()),
            title: map
                .get("xesam:title")
                .map_or(String::new(), |value| value.to_string()),

            auto_rating: map
                .get("xesam:autoRating")
                .map_or(0.0, |value| value.downcast_ref::<f64>().unwrap_or(0.0)),
            user_rating: map
                .get("xesam:userRating")
                .map_or(0.0, |value| value.downcast_ref::<f64>().unwrap_or(0.0)),

            bpm: map
                .get("xesam:audioBPM")
                .map_or(0, |value| value.downcast_ref::<i64>().unwrap_or(0)),

            disc_number: map
                .get("xesam:discNumber")
                .map_or(0, |value| value.downcast_ref::<i64>().unwrap_or(0)),
            track_number: map
                .get("xesam:trackNumber")
                .map_or(0, |value| value.downcast_ref::<i64>().unwrap_or(0)),
            use_count: map
                .get("xesam:useCount")
                .map_or(0, |value| value.downcast_ref::<i64>().unwrap_or(0)),

            created: map
                .get("mpris:contentCreated")
                .map_or(String::new(), |value| value.to_string()),
            first_used: map
                .get("mpris:firstUsed")
                .map_or(String::new(), |value| value.to_string()),
            last_used: map
                .get("mpris:lastUsed")
                .map_or(String::new(), |value| value.to_string()),
        }
    }
}
impl From<HashMap<String, OwnedValue>> for Metadata {
    fn from(value: HashMap<String, OwnedValue>) -> Self {
        Self::new_from_hashmap(value)
    }
}
