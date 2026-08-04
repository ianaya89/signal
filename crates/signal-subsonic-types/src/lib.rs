//! `OpenSubsonic` wire DTOs, shared by `signal-server` (which serializes them)
//! and `signal-subsonic-client` (which deserializes them).
//!
//! This crate models a *foreign* protocol, not Signal's domain — that's why it
//! lives outside `signal-core`. Ids are opaque `String`s here: Signal's server
//! encodes them as `tr-7`/`al-3` (see `signal-server`'s `Sid`), a remote server
//! may use bare integers or UUIDs, and neither side may assume the other's
//! scheme.
//!
//! The two directions are asymmetric: Signal's server populates every field it
//! emits, but Navidrome/Airsonic/Gonic disagree on which optional fields they
//! send. Hence `#[serde(default)]` throughout — it costs the serializing side
//! nothing and buys the parsing side tolerance for what it will actually meet.

use serde::{Deserialize, Serialize};

/// A song (or, in folder-browsing responses Signal doesn't emit, a directory).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Child {
    #[serde(default)]
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    #[serde(default)]
    pub is_dir: bool,
    #[serde(default)]
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub album: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artist: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub track: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disc_number: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub year: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub genre: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cover_art: Option<String>,
    #[serde(default)]
    pub size: u64,
    #[serde(default)]
    pub content_type: String,
    #[serde(default)]
    pub suffix: String,
    /// Seconds, per the Subsonic spec — not milliseconds.
    #[serde(default)]
    pub duration: u64,
    #[serde(default)]
    pub bit_rate: u32,
    /// Synthesized virtual path — never a real filesystem location.
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub play_count: u32,
    #[serde(default)]
    pub created: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starred: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub album_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artist_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_rating: Option<u8>,
    /// `music`, or the `podcast`/`video` kinds Signal neither emits nor plays.
    #[serde(rename = "type", default = "default_kind")]
    pub kind: String,
    #[serde(default)]
    pub is_video: bool,
}

fn default_kind() -> String {
    "music".to_owned()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumID3 {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub artist: String,
    #[serde(default)]
    pub artist_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cover_art: Option<String>,
    #[serde(default)]
    pub song_count: u32,
    /// Seconds.
    #[serde(default)]
    pub duration: u64,
    #[serde(default)]
    pub created: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub year: Option<i32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtistID3 {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub album_count: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cover_art: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Playlist {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub song_count: usize,
    /// Seconds.
    #[serde(default)]
    pub duration: u64,
    #[serde(default)]
    pub public: bool,
    #[serde(default)]
    pub owner: String,
    #[serde(default)]
    pub created: String,
    #[serde(default)]
    pub changed: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub entry: Vec<Child>,
}

/// `getArtists`/`getIndexes` payload.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtistsIndex {
    #[serde(default)]
    pub ignored_articles: String,
    #[serde(default)]
    pub index: Vec<IndexBucket>,
}

/// One initial-letter bucket of [`ArtistsIndex`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexBucket {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub artist: Vec<ArtistID3>,
}

/// `getArtist` payload: the artist's own attributes plus its albums.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtistWithAlbums {
    #[serde(flatten)]
    pub artist: ArtistID3,
    #[serde(default)]
    pub album: Vec<AlbumID3>,
}

/// `getAlbum` payload: the album's own attributes plus its songs.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumWithSongs {
    #[serde(flatten)]
    pub album: AlbumID3,
    #[serde(default)]
    pub song: Vec<Child>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult3 {
    #[serde(default)]
    pub artist: Vec<ArtistID3>,
    #[serde(default)]
    pub album: Vec<AlbumID3>,
    #[serde(default)]
    pub song: Vec<Child>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiErrorDto {
    #[serde(default)]
    pub code: u32,
    #[serde(default)]
    pub message: String,
}

/// Subsonic error codes worth acting on rather than just reporting.
impl ApiErrorDto {
    #[must_use]
    pub fn is_auth_failure(&self) -> bool {
        // 40 wrong credentials, 41 token auth not supported for this user
        matches!(self.code, 40 | 41)
    }
}

/// The outer `{"subsonic-response": {...}}` wrapper every JSON reply carries.
#[derive(Debug, Clone, Deserialize)]
pub struct Envelope {
    #[serde(rename = "subsonic-response")]
    pub response: ResponseBody,
}

/// Envelope contents. The endpoint-specific payload is kept as raw JSON under
/// its own key rather than typed here, because one struct can't be generic over
/// *which* key a given endpoint uses — call [`ResponseBody::take`] for that.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseBody {
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub version: String,
    #[serde(default, rename = "type")]
    pub server_type: Option<String>,
    #[serde(default)]
    pub server_version: Option<String>,
    #[serde(default)]
    pub open_subsonic: bool,
    #[serde(default)]
    pub error: Option<ApiErrorDto>,
    #[serde(flatten)]
    pub payload: serde_json::Map<String, serde_json::Value>,
}

impl ResponseBody {
    #[must_use]
    pub fn is_ok(&self) -> bool {
        self.error.is_none() && self.status != "failed"
    }

    /// Deserializes the payload stored under `key`.
    ///
    /// `Ok(None)` means the server answered successfully but omitted the key —
    /// legitimate for endpoints whose payload is optional (an empty `getArtists`
    /// on a fresh library), so it's distinct from a parse failure.
    ///
    /// # Errors
    /// Fails when the key is present but doesn't match `T`'s shape.
    pub fn take<T: serde::de::DeserializeOwned>(
        &self,
        key: &str,
    ) -> Result<Option<T>, serde_json::Error> {
        match self.payload.get(key) {
            Some(value) => serde_json::from_value(value.clone()).map(Some),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn child_tolerates_a_minimal_third_party_song() {
        // Gonic-style: only the fields it actually knows
        let c: Child = serde_json::from_str(r#"{"id":"42","title":"Persiana Americana"}"#).unwrap();
        assert_eq!(c.id, "42");
        assert_eq!(c.kind, "music");
        assert_eq!(c.duration, 0);
        assert!(c.album_id.is_none());
    }

    #[test]
    fn child_round_trips_without_emitting_nulls() {
        let c = Child {
            id: "tr-7".into(),
            title: "En la Ciudad de la Furia".into(),
            ..Child::default()
        };
        let v = serde_json::to_value(&c).unwrap();
        assert!(v.as_object().unwrap().values().all(|x| !x.is_null()));
        let back: Child = serde_json::from_value(v).unwrap();
        assert_eq!(back.id, "tr-7");
    }

    #[test]
    fn flattened_album_keeps_its_own_attrs_and_songs() {
        let a: AlbumWithSongs = serde_json::from_str(
            r#"{"id":"al-3","name":"Doble Vida","songCount":2,"song":[{"id":"tr-7"},{"id":"tr-8"}]}"#,
        )
        .unwrap();
        assert_eq!(a.album.id, "al-3");
        assert_eq!(a.album.song_count, 2);
        assert_eq!(a.song.len(), 2);
    }

    #[test]
    fn envelope_reports_failure_and_extracts_payload() {
        let ok: Envelope = serde_json::from_str(
            r#"{"subsonic-response":{"status":"ok","version":"1.16.1","openSubsonic":true,
                "artists":{"ignoredArticles":"","index":[{"name":"S","artist":[{"id":"ar-1","name":"Soda Stereo","albumCount":7}]}]}}}"#,
        )
        .unwrap();
        assert!(ok.response.is_ok());
        let idx: ArtistsIndex = ok.response.take("artists").unwrap().unwrap();
        assert_eq!(idx.index[0].artist[0].name, "Soda Stereo");
        assert!(ok.response.take::<ArtistsIndex>("nope").unwrap().is_none());

        let bad: Envelope = serde_json::from_str(
            r#"{"subsonic-response":{"status":"failed","version":"1.16.1",
                "error":{"code":40,"message":"wrong username or password"}}}"#,
        )
        .unwrap();
        assert!(!bad.response.is_ok());
        assert!(bad.response.error.unwrap().is_auth_failure());
    }
}
