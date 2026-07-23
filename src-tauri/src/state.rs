use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use signal_core::{AppConfig, EventBus};
use signal_db::DbPool;
use signal_player::Player;

/// Arc-free by design: Tauri's `State` wraps this in an Arc already.
pub struct AppState {
    pub config: AppConfig,
    pub events: EventBus,
    pub db: DbPool,
    pub player: Player,
    /// Guards against concurrent library scans.
    pub scanning: Arc<AtomicBool>,
}
