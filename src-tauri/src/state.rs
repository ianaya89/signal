use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use signal_core::{AppConfig, EventBus};
use signal_db::DbPool;
use signal_player::Player;
use signal_plugins::PluginHost;
use signal_scanner::{Excludes, Scanner, WatcherHandle};

/// Arc-free by design: Tauri's `State` wraps this in an Arc already.
pub struct AppState {
    pub config: AppConfig,
    pub events: EventBus,
    pub db: DbPool,
    pub player: Player,
    /// Guards against concurrent library scans.
    pub scanning: Arc<AtomicBool>,
    /// Set by the doctor's cancel button; the artwork batch checks it between
    /// albums (each one costs a full second of `MusicBrainz` throttling).
    pub artwork_cancel: Arc<AtomicBool>,
    /// Guards against concurrent audio authenticity analyses.
    pub analyzing: Arc<AtomicBool>,
    /// Doctor's stop button; polled between tracks and inside decode loops.
    pub analysis_cancel: Arc<AtomicBool>,
    /// Live fs watchers, one per library root; replaced together.
    pub watcher: Mutex<Vec<WatcherHandle>>,
    /// Running `OpenSubsonic` server, if any. Lock → take → drop guard →
    /// `stop().await`; never hold across an await.
    pub server: Mutex<Option<signal_server::ServerHandle>>,
    /// Path substrings excluded from scans (config.toml `[library] exclude`).
    pub excludes: Excludes,
    /// Write metadata edits back into audio file tags
    /// (config.toml `[library] write_tags`).
    pub write_tags: Arc<std::sync::atomic::AtomicBool>,
    /// Implicit play order (album/list the current track came from). The
    /// queue always takes priority over it when advancing.
    pub play_context: Mutex<PlayContext>,
    pub play_mode: Mutex<PlayMode>,
    pub plugins: Arc<PluginHost>,
    /// Recently played track ids, newest last (drives `player_prev`).
    pub play_history: Mutex<Vec<i64>>,
    /// One `OpenSubsonic` client per `remote_sources.id`, so per-source TLS
    /// settings and the connection pool survive across requests. Dropped from
    /// the map whenever the row is edited or removed.
    pub remote_clients: Mutex<HashMap<i64, signal_subsonic_client::SubsonicClient>>,
    /// Remote songs that have been handed to the player, keyed by the negative
    /// id standing in for their missing `tracks` row.
    pub remote_tracks: Mutex<RemoteSlab>,
}

pub const HISTORY_CAP: usize = 100;

/// A remote song the player has been pointed at.
///
/// Carries the metadata the now-playing UI needs, because there is no `tracks`
/// row to read it back from (`docs/11-subsonic-client.md` §2.3).
#[derive(Debug, Clone)]
pub struct RemoteTrack {
    pub source_id: i64,
    pub remote_id: String,
    pub url: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: u64,
    pub suffix: String,
    pub bitrate_kbps: u32,
}

/// Negative-id registry for remote songs.
///
/// The whole advance path — `PlayContext`, `play_history`, `next_candidate` —
/// speaks `i64` and never asks what the id means. Handing remote songs ids from
/// a disjoint (negative) range lets them ride that machinery unchanged; the
/// only places that must branch are the two that resolve an id to something
/// playable, plus the now-playing lookup.
#[derive(Default)]
pub struct RemoteSlab {
    next: i64,
    by_id: HashMap<i64, RemoteTrack>,
    by_remote: HashMap<(i64, String), i64>,
}

impl RemoteSlab {
    /// Returns the id for `track`, reusing it if the same remote song is
    /// already registered. Re-registering keeps a replayed album from growing
    /// the map, so it stays bounded by distinct remote songs touched this run
    /// rather than by play count. Entries are never evicted: `play_history`
    /// holds ids indefinitely, and reusing one would resume the wrong song.
    pub fn register(&mut self, track: RemoteTrack) -> i64 {
        let key = (track.source_id, track.remote_id.clone());
        if let Some(&id) = self.by_remote.get(&key) {
            self.by_id.insert(id, track);
            return id;
        }
        self.next -= 1;
        let id = self.next;
        self.by_remote.insert(key, id);
        self.by_id.insert(id, track);
        id
    }

    #[must_use]
    pub fn get(&self, id: i64) -> Option<&RemoteTrack> {
        self.by_id.get(&id)
    }
}

/// True for ids handed out by [`RemoteSlab`]. Real tracks come from `SQLite`
/// `AUTOINCREMENT`, so the ranges cannot overlap.
#[must_use]
pub fn is_remote_id(track_id: i64) -> bool {
    track_id < 0
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Repeat {
    #[default]
    Off,
    All,
    One,
}

#[derive(Debug, Clone, Copy, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayMode {
    pub shuffle: bool,
    pub repeat: Repeat,
}

#[derive(Default)]
pub struct PlayContext {
    pub track_ids: Vec<i64>,
    pub position: usize,
    /// Tracks already visited this shuffle round — standard shuffle plays
    /// everything once before repeating.
    played: std::collections::HashSet<i64>,
}

impl PlayContext {
    /// Track that should follow the current one, honoring shuffle/repeat.
    /// Shuffle exhausts unplayed tracks first; repeat-all restarts the
    /// round (or wraps, in linear order) at the end.
    pub fn peek_next(&self, mode: PlayMode) -> Option<i64> {
        if self.track_ids.is_empty() {
            return None;
        }
        if mode.shuffle && self.track_ids.len() > 1 {
            let current = self.track_ids.get(self.position).copied();
            let unplayed: Vec<i64> = self
                .track_ids
                .iter()
                .copied()
                .filter(|id| !self.played.contains(id) && Some(*id) != current)
                .collect();
            let pool = if unplayed.is_empty() {
                if mode.repeat != Repeat::All {
                    return None; // round exhausted
                }
                self.track_ids
                    .iter()
                    .copied()
                    .filter(|id| Some(*id) != current)
                    .collect()
            } else {
                unplayed
            };
            // cheap jitter; no external RNG dependency
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(7, |d| d.subsec_nanos() as usize);
            return pool.get(nanos % pool.len()).copied();
        }
        match self.track_ids.get(self.position + 1) {
            Some(&id) => Some(id),
            None if mode.repeat == Repeat::All => self.track_ids.first().copied(),
            None => None,
        }
    }

    /// Moves onto `track_id` wherever it sits in the context; true on hit.
    pub fn jump_to(&mut self, track_id: i64) -> bool {
        if let Some(idx) = self.track_ids.iter().position(|&id| id == track_id) {
            // leaving the current track marks it visited for this round
            if let Some(&current) = self.track_ids.get(self.position) {
                self.played.insert(current);
            }
            if self.played.len() >= self.track_ids.len() {
                self.played.clear(); // new round
            }
            self.played.insert(track_id);
            self.position = idx;
            true
        } else {
            false
        }
    }
}

impl AppState {
    pub fn scanner(&self) -> Scanner {
        Scanner::new(
            self.db.clone(),
            self.events.clone(),
            self.config.cache_dir.clone(),
            self.excludes.clone(),
        )
    }

    /// Replaces all filesystem watchers with one per root.
    pub fn start_watchers(&self, roots: &[std::path::PathBuf]) {
        let mut handles = Vec::with_capacity(roots.len());
        for root in roots {
            match signal_scanner::spawn_watcher(
                self.scanner(),
                root,
                tokio::runtime::Handle::current(),
            ) {
                Ok(handle) => handles.push(handle),
                Err(err) => tracing::warn!(root = %root.display(), "watcher start failed: {err}"),
            }
        }
        if let Ok(mut guard) = self.watcher.lock() {
            *guard = handles;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{is_remote_id, RemoteSlab, RemoteTrack};

    fn track(source_id: i64, remote_id: &str, title: &str) -> RemoteTrack {
        RemoteTrack {
            source_id,
            remote_id: remote_id.to_owned(),
            url: format!("http://host/rest/stream?id={remote_id}"),
            title: title.to_owned(),
            artist: "Soda Stereo".to_owned(),
            album: "Doble Vida".to_owned(),
            duration_ms: 300_000,
            suffix: "flac".to_owned(),
            bitrate_kbps: 1_024,
        }
    }

    #[test]
    fn ids_stay_out_of_the_local_track_range() {
        let mut slab = RemoteSlab::default();
        let a = slab.register(track(1, "tr-7", "En la Ciudad de la Furia"));
        let b = slab.register(track(1, "tr-8", "Persiana Americana"));
        assert!(is_remote_id(a) && is_remote_id(b));
        assert_ne!(a, b);
        // real ids come from AUTOINCREMENT, which starts at 1
        assert!(!is_remote_id(1));
    }

    #[test]
    fn re_registering_the_same_song_reuses_its_id() {
        let mut slab = RemoteSlab::default();
        let first = slab.register(track(1, "tr-7", "old title"));
        let again = slab.register(track(1, "tr-7", "renamed upstream"));
        assert_eq!(first, again);
        // the newer metadata wins, so a rename upstream shows up next play
        assert_eq!(
            slab.get(first).map(|t| t.title.as_str()),
            Some("renamed upstream")
        );
    }

    #[test]
    fn the_same_remote_id_on_two_servers_stays_distinct() {
        let mut slab = RemoteSlab::default();
        let one = slab.register(track(1, "42", "from server one"));
        let two = slab.register(track(2, "42", "from server two"));
        assert_ne!(one, two);
        assert_eq!(slab.get(two).map(|t| t.source_id), Some(2));
    }

    #[test]
    fn unknown_ids_resolve_to_nothing() {
        let slab = RemoteSlab::default();
        assert!(slab.get(-1).is_none());
    }
}
