//! Where a piece of audio comes from.
//!
//! Signal plays two kinds of thing: files it scanned itself, and streams from
//! a remote `OpenSubsonic` server (`docs/11-subsonic-client.md`). The two are
//! kept apart at the type level because they differ in identity — a local
//! track is a `tracks.id`, a remote one is an opaque id that only means
//! anything relative to the server that issued it.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// What to hand mpv. `loadfile` treats a path and a URL identically — ffmpeg's
/// protocol layer resolves both — so nothing below this type needs to branch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind", content = "value")]
pub enum MediaSource {
    File(PathBuf),
    Url(String),
}

impl MediaSource {
    /// The string mpv actually receives.
    #[must_use]
    pub fn as_mpv_target(&self) -> std::borrow::Cow<'_, str> {
        match self {
            Self::File(path) => path.to_string_lossy(),
            Self::Url(url) => std::borrow::Cow::Borrowed(url),
        }
    }

    #[must_use]
    pub fn is_remote(&self) -> bool {
        matches!(self, Self::Url(_))
    }
}

impl From<PathBuf> for MediaSource {
    fn from(path: PathBuf) -> Self {
        Self::File(path)
    }
}

/// A track on a remote server: opaque id, scoped by the source that issued it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteTrackRef {
    /// `remote_sources.id`.
    pub source_id: i64,
    pub remote_id: String,
}

/// Which library a queue entry or play request refers to.
///
/// Remote entries carry no `tracks.id` because they have no row in `tracks` —
/// `tracks.file_path` is `NOT NULL UNIQUE` and every consumer relies on it
/// naming a real file on disk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum PlaybackSource {
    Local {
        track_id: i64,
    },
    #[serde(rename_all = "camelCase")]
    Remote {
        source_id: i64,
        remote_id: String,
    },
}

impl PlaybackSource {
    /// `Some` only for local tracks — the id everything keyed on `tracks.id`
    /// (history, ratings, analysis) needs, and deliberately absent otherwise.
    #[must_use]
    pub fn local_track_id(&self) -> Option<i64> {
        match self {
            Self::Local { track_id } => Some(*track_id),
            Self::Remote { .. } => None,
        }
    }
}

impl From<RemoteTrackRef> for PlaybackSource {
    fn from(r: RemoteTrackRef) -> Self {
        Self::Remote {
            source_id: r.source_id,
            remote_id: r.remote_id,
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn mpv_target_is_the_raw_string_either_way() {
        assert_eq!(
            MediaSource::File(PathBuf::from("/music/a b.flac")).as_mpv_target(),
            "/music/a b.flac"
        );
        assert_eq!(
            MediaSource::Url("http://nas:4533/rest/stream?id=7".to_owned()).as_mpv_target(),
            "http://nas:4533/rest/stream?id=7"
        );
    }

    #[test]
    fn playback_source_only_yields_a_track_id_when_local() {
        assert_eq!(
            PlaybackSource::Local { track_id: 7 }.local_track_id(),
            Some(7)
        );
        let remote = PlaybackSource::from(RemoteTrackRef {
            source_id: 2,
            remote_id: "abc".to_owned(),
        });
        assert_eq!(remote.local_track_id(), None);
    }

    #[test]
    fn playback_source_round_trips_through_its_ipc_shape() {
        let remote = PlaybackSource::Remote {
            source_id: 2,
            remote_id: "abc".to_owned(),
        };
        let json = serde_json::to_string(&remote).unwrap();
        assert!(json.contains("\"kind\":\"remote\""), "{json}");
        assert!(json.contains("\"remoteId\":\"abc\""), "{json}");
        assert_eq!(
            serde_json::from_str::<PlaybackSource>(&json).unwrap(),
            remote
        );
    }
}
