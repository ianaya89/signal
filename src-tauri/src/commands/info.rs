use serde::Serialize;
use signal_core::SignalError;
use tauri::State;

use crate::commands::DbResultExt;
use crate::state::AppState;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub db_path: String,
    pub cache_dir: String,
    pub library_root: Option<String>,
    pub track_count: i64,
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn app_info(state: State<'_, AppState>) -> Result<AppInfo, SignalError> {
    Ok(AppInfo {
        version: env!("CARGO_PKG_VERSION").to_owned(),
        db_path: state.config.db_path.to_string_lossy().into_owned(),
        cache_dir: state.config.cache_dir.to_string_lossy().into_owned(),
        library_root: state.db.settings().get("library.root").await.db_err()?,
        track_count: state.db.tracks().count().await.db_err()?,
    })
}
