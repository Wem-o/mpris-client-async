use std::{collections::HashMap, ops::Deref, time::Duration};

use zbus::zvariant::{ObjectPath, OwnedValue, Value};

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct TrackId(ObjectPath<'static>);
impl TrackId {
    /// A special track ID to indicate "no track".
    pub const NO_TRACK: TrackId = TrackId(ObjectPath::from_static_str_unchecked(
        "/org/mpris/MediaPlayer2/TrackList/NoTrack",
    ));

    /// Returns the track ID as an [`ObjectPath`].
    pub fn into_inner(self) -> ObjectPath<'static> {
        self.0
    }
}
impl<'a> From<ObjectPath<'a>> for TrackId {
    fn from(value: ObjectPath<'a>) -> Self {
        Self(value.into_owned())
    }
}
impl Deref for TrackId {
    type Target = ObjectPath<'static>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl From<TrackId> for ObjectPath<'static> {
    fn from(o: TrackId) -> Self {
        o.into_inner()
    }
}
impl From<TrackId> for Value<'_> {
    fn from(o: TrackId) -> Self {
        o.into_inner().into()
    }
}

/// Metadata of a media
///
/// It's construced from the [metadata specs](https://www.freedesktop.org/wiki/Specifications/mpris-spec/metadata/).
///
/// Dont assume any of this is actually provided (other than trackid, which might also be just '/org/mpris/MediaPlayer2/TrackList/NoTrack').
#[derive(Debug, Clone, PartialEq)]
pub struct Metadata {
    // MPRIS specific things
    /// A unique identity for this track within the context of an MPRIS object.
    ///
    /// This (according to the specs) is always provided.
    /// (Dont rely on it too much tho)
    pub trackid: TrackId,
    /// The length of the track
    pub length: Option<Duration>,
    /// The URI of the location of the track. You should not assume this will exist when a new track is played.
    ///
    /// Local files will start "file://", but it can be an online URL as well (for example Spotify's desktop player provides a URL).
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
    pub disc_number: i32,
    /// The track number on the album disc
    pub track_number: i32,
    /// The location of the file
    pub url: String,

    /// The genres of the media
    pub genres: Vec<String>,
    /// The lyricists of the media
    pub lyricists: Vec<String>,

    /// The time when the media was created. Should follow ISO 8601.
    ///
    /// xesam:contentCreated
    pub created: String,
    /// The time when the was first played. Should follow ISO 8601.
    pub first_used: String,
    /// The time when the was last played. Should follow ISO 8601.
    pub last_used: String,
    /// The number of times the track has been played
    pub use_count: i32,
}
impl Metadata {
    pub fn new_from_hashmap(map: HashMap<String, OwnedValue>) -> Self {
        // dbg!(&map);
        Self {
            trackid: match map.get("mpris:trackid") {
                Some(id) => id
                    .downcast_ref::<ObjectPath>()
                    .unwrap_or(TrackId::NO_TRACK.into_inner())
                    .into(),
                None => TrackId::NO_TRACK,
            },

            length: map.get("mpris:length").map_or(None, |value| {
                value
                    .downcast_ref::<i64>()
                    .ok()
                    .map(|d| Duration::from_micros(d as u64))
            }),

            art_url: map
                .get("mpris:artUrl")
                .map_or(None, |value| Some(value.downcast_ref::<String>().unwrap())),

            album: map.get("xesam:album").map_or(String::new(), |value| {
                value.downcast_ref::<String>().unwrap_or(String::new())
            }),

            album_artist: map.get("xesam:albumArtist").map_or(Vec::new(), |value| {
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

            lyrics: map.get("xesam:asText").map_or(String::new(), |value| {
                value.downcast_ref::<String>().unwrap_or(String::new())
            }),

            url: map.get("mpris:url").map_or(String::new(), |value| {
                value.downcast_ref::<String>().unwrap_or(String::new())
            }),

            title: map.get("xesam:title").map_or(String::new(), |value| {
                value.downcast_ref::<String>().unwrap_or(String::new())
            }),

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
                .map_or(0, |value| value.downcast_ref::<i32>().unwrap_or(0)),

            track_number: map
                .get("xesam:trackNumber")
                .map_or(0, |value| value.downcast_ref::<i32>().unwrap_or(0)),

            use_count: map
                .get("xesam:useCount")
                .map_or(0, |value| value.downcast_ref::<i32>().unwrap_or(0)),

            created: map
                .get("xesam:contentCreated")
                .map_or(String::new(), |value| {
                    value.downcast_ref::<String>().unwrap_or(String::new())
                }),

            first_used: map.get("xesam:firstUsed").map_or(String::new(), |value| {
                value.downcast_ref::<String>().unwrap_or(String::new())
            }),

            last_used: map.get("xesam:lastUsed").map_or(String::new(), |value| {
                value.downcast_ref::<String>().unwrap_or(String::new())
            }),
        }
    }
}
impl From<HashMap<String, OwnedValue>> for Metadata {
    fn from(value: HashMap<String, OwnedValue>) -> Self {
        Self::new_from_hashmap(value)
    }
}
