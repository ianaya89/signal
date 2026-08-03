//! Subsonic wire DTOs and the Track/Album/Artist mappers. Every struct
//! serializes camelCase with optionals skipped so the XML walker never
//! sees nulls.

use std::collections::HashMap;

use chrono::SecondsFormat;
use serde::Serialize;
use signal_core::{AlbumSummary, ArtistSummary, Track};

use crate::ids::Sid;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Child {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    pub is_dir: bool,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub album: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artist: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disc_number: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_art: Option<String>,
    pub size: u64,
    pub content_type: String,
    pub suffix: String,
    pub duration: u64,
    pub bit_rate: u32,
    /// Synthesized virtual path — never the real filesystem location.
    pub path: String,
    pub play_count: u32,
    pub created: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub starred: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub album_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artist_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_rating: Option<u8>,
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub is_video: bool,
}

impl Child {
    pub fn from_track(
        track: &Track,
        artists: &HashMap<i64, String>,
        albums: &HashMap<i64, String>,
    ) -> Self {
        let artist = artists.get(&track.artist_id).cloned();
        let album = albums.get(&track.album_id).cloned();
        let has_album = track.album_id != 0;
        let suffix = suffix_of(&track.technical.file_path);
        let created = track.added_at.to_rfc3339_opts(SecondsFormat::Secs, true);
        let path = format!(
            "{}/{}/{:02}-{:02} {}.{}",
            artist.as_deref().unwrap_or("Unknown Artist"),
            album.as_deref().unwrap_or("Unknown Album"),
            track.disc_no.unwrap_or(1),
            track.track_no.unwrap_or(0),
            track.title,
            suffix,
        );
        Self {
            id: Sid::Track(track.id).to_string(),
            parent: has_album.then(|| Sid::Album(track.album_id).to_string()),
            is_dir: false,
            title: track.title.clone(),
            album,
            artist,
            track: track.track_no,
            disc_number: track.disc_no,
            year: track.year,
            cover_art: has_album.then(|| Sid::Album(track.album_id).to_string()),
            size: track.technical.file_size_bytes,
            content_type: content_type_of(&suffix).to_owned(),
            suffix,
            duration: track.duration_ms / 1_000,
            bit_rate: track.technical.bitrate_kbps,
            path,
            play_count: track.play_count,
            created: created.clone(),
            starred: track.favorite.then_some(created),
            album_id: has_album.then(|| Sid::Album(track.album_id).to_string()),
            artist_id: Some(Sid::Artist(track.artist_id).to_string()),
            user_rating: track.rating.filter(|r| *r > 0),
            kind: "music",
            is_video: false,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AlbumID3 {
    pub id: String,
    pub name: String,
    pub artist: String,
    pub artist_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_art: Option<String>,
    pub song_count: u32,
    pub duration: u64,
    pub created: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year: Option<i32>,
}

impl AlbumID3 {
    pub fn from_summary(album: &AlbumSummary, durations: &HashMap<i64, i64>) -> Self {
        #[allow(clippy::cast_sign_loss)]
        let duration_secs = durations.get(&album.id).copied().unwrap_or(0).max(0) as u64 / 1_000;
        Self {
            id: Sid::Album(album.id).to_string(),
            name: album.name.clone(),
            artist: album.artist_name.clone(),
            artist_id: Sid::Artist(album.artist_id).to_string(),
            cover_art: album
                .artwork_path
                .is_some()
                .then(|| Sid::Album(album.id).to_string()),
            song_count: album.track_count,
            duration: duration_secs,
            created: album.added_at.clone(),
            year: album.year,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ArtistID3 {
    pub id: String,
    pub name: String,
    pub album_count: u32,
}

impl ArtistID3 {
    pub fn from_summary(artist: &ArtistSummary) -> Self {
        Self {
            id: Sid::Artist(artist.id).to_string(),
            name: artist.name.clone(),
            album_count: artist.album_count,
        }
    }
}

pub(crate) fn suffix_of(path: &std::path::Path) -> String {
    path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("bin")
        .to_ascii_lowercase()
}

pub(crate) fn content_type_of(suffix: &str) -> &'static str {
    match suffix {
        "flac" => "audio/flac",
        "mp3" => "audio/mpeg",
        "m4a" | "mp4" | "aac" => "audio/mp4",
        "ogg" => "audio/ogg",
        "opus" => "audio/opus",
        "wav" => "audio/wav",
        "aif" | "aiff" => "audio/aiff",
        _ => "application/octet-stream",
    }
}

/// serde → `serde_json::Value`, for handing DTOs to the envelope.
pub(crate) fn to_value(v: impl Serialize) -> serde_json::Value {
    serde_json::to_value(v).unwrap_or(serde_json::Value::Null)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    fn track() -> Track {
        Track {
            id: 7,
            title: "En la Ciudad de la Furia".into(),
            artist_id: 1,
            album_id: 3,
            track_no: Some(4),
            disc_no: Some(1),
            year: Some(1988),
            duration_ms: 285_500,
            rating: Some(5),
            favorite: true,
            play_count: 12,
            skip_count: 0,
            added_at: chrono::DateTime::parse_from_rfc3339("2026-01-15T10:00:00Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
            modified_at: chrono::Utc::now(),
            last_played_at: None,
            technical: signal_core::TrackTechnical {
                codec: "FLAC".into(),
                container: "FLAC".into(),
                bitrate_kbps: 1_024,
                bit_depth: Some(16),
                sample_rate_hz: 44_100,
                channels: 2,
                replaygain_track_gain: None,
                replaygain_album_gain: None,
                peak: None,
                dr_score: None,
                encoder: None,
                file_path: "/music/soda/doble vida/04 furia.flac".into(),
                file_size_bytes: 31_000_000,
                md5: None,
            },
        }
    }

    #[test]
    fn child_mapping() {
        let artists = HashMap::from([(1_i64, "Soda Stereo".to_owned())]);
        let albums = HashMap::from([(3_i64, "Doble Vida".to_owned())]);
        let c = Child::from_track(&track(), &artists, &albums);
        assert_eq!(c.id, "tr-7");
        assert_eq!(c.album_id.as_deref(), Some("al-3"));
        assert_eq!(c.artist.as_deref(), Some("Soda Stereo"));
        assert_eq!(c.duration, 285);
        assert_eq!(c.suffix, "flac");
        assert_eq!(c.content_type, "audio/flac");
        assert!(c.starred.is_some());
        assert_eq!(c.user_rating, Some(5));
        assert!(!c.path.contains("/music/"), "real path leaked: {}", c.path);

        // XML-ready: no nulls once serialized
        let v = to_value(&c);
        assert!(v.as_object().unwrap().values().all(|x| !x.is_null()));
    }

    #[test]
    fn albumless_track_omits_album_fields() {
        let mut t = track();
        t.album_id = 0;
        t.favorite = false;
        let c = Child::from_track(&t, &HashMap::new(), &HashMap::new());
        assert!(c.album_id.is_none());
        assert!(c.cover_art.is_none());
        assert!(c.starred.is_none());
        assert_eq!(c.artist.as_deref(), None);
    }
}
