use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use signal_core::{AppConfig, EventBus};
use signal_db::DbPool;
use signal_player::Player;
use signal_plugins::PluginHost;
use signal_scanner::{Scanner, WatcherHandle};

/// Arc-free by design: Tauri's `State` wraps this in an Arc already.
pub struct AppState {
    pub config: AppConfig,
    pub events: EventBus,
    pub db: DbPool,
    pub player: Player,
    /// Guards against concurrent library scans.
    pub scanning: Arc<AtomicBool>,
    /// Live fs watcher on the library root; replaced when the root changes.
    pub watcher: Mutex<Option<WatcherHandle>>,
    /// Implicit play order (album/list the current track came from). The
    /// queue always takes priority over it when advancing.
    pub play_context: Mutex<PlayContext>,
    pub play_mode: Mutex<PlayMode>,
    pub plugins: Arc<PluginHost>,
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
}

impl PlayContext {
    /// Track that should follow the current one, honoring shuffle/repeat.
    /// Shuffle picks pseudo-randomly (excluding the current position);
    /// repeat-all wraps at the end.
    pub fn peek_next(&self, mode: PlayMode) -> Option<i64> {
        if self.track_ids.is_empty() {
            return None;
        }
        if mode.shuffle && self.track_ids.len() > 1 {
            // cheap deterministic-ish jitter; no external RNG dependency
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(7, |d| d.subsec_nanos() as usize);
            let mut idx = nanos % self.track_ids.len();
            if idx == self.position {
                idx = (idx + 1) % self.track_ids.len();
            }
            return self.track_ids.get(idx).copied();
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
            self.position = idx;
            true
        } else {
            false
        }
    }
}

impl AppState {
    /// (Re)starts the filesystem watcher on `root`.
    pub fn start_watcher(&self, root: &std::path::Path) {
        let scanner = Scanner::new(
            self.db.clone(),
            self.events.clone(),
            self.config.cache_dir.clone(),
        );
        match signal_scanner::spawn_watcher(scanner, root, tokio::runtime::Handle::current()) {
            Ok(handle) => {
                if let Ok(mut guard) = self.watcher.lock() {
                    *guard = Some(handle);
                }
            }
            Err(err) => tracing::warn!("watcher start failed: {err}"),
        }
    }
}
