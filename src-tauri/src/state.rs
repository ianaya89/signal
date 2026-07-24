use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use signal_core::{AppConfig, EventBus};
use signal_db::DbPool;
use signal_player::Player;
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
}

#[derive(Default)]
pub struct PlayContext {
    pub track_ids: Vec<i64>,
    pub position: usize,
}

impl PlayContext {
    /// Track that should follow the current one within the context.
    pub fn peek_next(&self) -> Option<i64> {
        self.track_ids.get(self.position + 1).copied()
    }

    /// Moves onto `track_id` if it is the next context entry; true on hit.
    pub fn advance_to(&mut self, track_id: i64) -> bool {
        if self.peek_next() == Some(track_id) {
            self.position += 1;
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
