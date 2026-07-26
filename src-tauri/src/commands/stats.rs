use signal_core::SignalError;
use signal_db::{Discover, StatsOverview};
use tauri::State;

use crate::commands::DbResultExt;
use crate::state::AppState;

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn stats_overview(state: State<'_, AppState>) -> Result<StatsOverview, SignalError> {
    state.db.stats().overview(365).await.db_err()
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn library_discover(state: State<'_, AppState>) -> Result<Discover, SignalError> {
    state.db.stats().discover(20).await.db_err()
}
