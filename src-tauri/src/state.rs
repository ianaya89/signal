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
    /// Live fs watchers, one per library root; replaced together.
    pub watcher: Mutex<Vec<WatcherHandle>>,
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
}

pub const HISTORY_CAP: usize = 100;

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
