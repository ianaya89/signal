use std::path::PathBuf;

/// App-wide filesystem locations. Resolved once at startup by `src-tauri`
/// and passed down; crates never guess paths on their own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppConfig {
    pub data_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub db_path: PathBuf,
}

impl AppConfig {
    #[must_use]
    pub fn new(data_dir: PathBuf, cache_dir: PathBuf) -> Self {
        let db_path = data_dir.join("signal.db");
        Self {
            data_dir,
            cache_dir,
            db_path,
        }
    }
}
