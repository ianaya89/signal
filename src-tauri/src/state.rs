use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use signal_core::{AppConfig, EventBus};
use signal_db::DbPool;

/// Arc-free by design: Tauri's `State` wraps this in an Arc already.
pub struct AppState {
    pub config: AppConfig,
    pub events: EventBus,
    pub db: DbPool,
    /// Guards against concurrent library scans.
    pub scanning: Arc<AtomicBool>,
}
