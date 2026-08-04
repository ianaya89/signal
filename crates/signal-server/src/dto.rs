//! Mappers from Signal's domain types onto the shared Subsonic wire DTOs.
//!
//! The structs themselves live in `signal-subsonic-types` so the client half
//! (`signal-subsonic-client`) parses exactly what this half emits. What stays
//! here is everything that knows about *Signal*: the [`Sid`] id scheme and the
//! `Track`/`AlbumSummary`/`ArtistSummary` lookups — a remote server's ids and
//! row shapes are none of the shared crate's business. Free functions rather
//! than inherent constructors, since the types are now foreign to this crate.

use std::collections::HashMap;

use chrono::SecondsFormat;
use serde::Serialize;
use signal_core::{AlbumSummary, ArtistSummary, Track};
use signal_subsonic_types::{AlbumID3, ArtistID3, Child, Playlist};

use crate::handlers::NameMaps;
use crate::ids::Sid;

pub(crate) fn child_from_track(track: &Track, maps: &NameMaps) -> Child {
    let artist = maps.artists.get(&track.artist_id).cloned();
    let album = maps.albums.get(&track.album_id).cloned();
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
    Child {
        id: Sid::Track(track.id).to_string(),
        parent: has_album.then(|| Sid::Album(track.album_id).to_string()),
        is_dir: false,
        title: track.title.clone(),
        album,
        artist,
        track: track.track_no,
        disc_number: track.disc_no,
        year: track.year,
        genre: maps.genres.get(&track.id).cloned(),
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
        kind: "music".to_owned(),
        is_video: false,
    }
}

pub(crate) fn album_from_summary(album: &AlbumSummary, durations: &HashMap<i64, i64>) -> AlbumID3 {
    #[allow(clippy::cast_sign_loss)]
    let duration_secs = durations.get(&album.id).copied().unwrap_or(0).max(0) as u64 / 1_000;
    AlbumID3 {
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

pub(crate) fn artist_from_summary(artist: &ArtistSummary) -> ArtistID3 {
    ArtistID3 {
        id: Sid::Artist(artist.id).to_string(),
        name: artist.name.clone(),
        album_count: artist.album_count,
        cover_art: None,
    }
}

pub(crate) fn playlist_attrs(
    id: &Sid,
    name: &str,
    song_count: usize,
    duration_secs: u64,
    stamp: Option<&(String, String)>,
) -> Playlist {
    let (created, changed) = match stamp {
        Some((created, changed)) => (created.clone(), changed.clone()),
        None => (String::new(), String::new()),
    };
    Playlist {
        id: id.to_string(),
        name: name.to_owned(),
        song_count,
        duration: duration_secs,
        public: false,
        owner: "signal".to_owned(),
        created,
        changed,
        entry: Vec::new(),
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

    fn maps() -> NameMaps {
        NameMaps {
            artists: HashMap::from([(1_i64, "Soda Stereo".to_owned())]),
            albums: HashMap::from([(3_i64, "Doble Vida".to_owned())]),
            genres: HashMap::from([(7_i64, "Rock Nacional".to_owned())]),
        }
    }

    #[test]
    fn child_mapping() {
        let c = child_from_track(&track(), &maps());
        assert_eq!(c.id, "tr-7");
        assert_eq!(c.album_id.as_deref(), Some("al-3"));
        assert_eq!(c.artist.as_deref(), Some("Soda Stereo"));
        assert_eq!(c.genre.as_deref(), Some("Rock Nacional"));
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
        let empty = NameMaps {
            artists: HashMap::new(),
            albums: HashMap::new(),
            genres: HashMap::new(),
        };
        let c = child_from_track(&t, &empty);
        assert!(c.album_id.is_none());
        assert!(c.cover_art.is_none());
        assert!(c.starred.is_none());
        assert!(c.genre.is_none());
        assert_eq!(c.artist.as_deref(), None);
    }

    #[test]
    fn playlist_without_entries_serializes_no_entry_key() {
        let p = playlist_attrs(&Sid::Playlist(2), "road trip", 3, 600, None);
        let v = to_value(&p);
        let obj = v.as_object().unwrap();
        assert_eq!(obj["id"], "pl-2");
        assert_eq!(obj["songCount"], 3);
        assert!(
            !obj.contains_key("entry"),
            "empty entry list leaked into XML"
        );
        assert!(obj.values().all(|x| !x.is_null()));
    }
}
