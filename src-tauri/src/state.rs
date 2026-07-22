use std::collections::HashMap;

use signal_core::EventBus;
use tokio::sync::RwLock;

/// Arc-free by design: Tauri's `State` wraps this in an Arc already.
pub struct AppState {
    pub events: EventBus,
    // M0: in-memory settings; replaced by the signal-db settings repo in M1.
    pub settings: RwLock<HashMap<String, String>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            events: EventBus::default(),
            settings: RwLock::new(HashMap::new()),
        }
    }
}
