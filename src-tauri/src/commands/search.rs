use signal_core::{SignalError, Track};
use tauri::State;

use crate::state::AppState;

const SEARCH_LIMIT: u32 = 200;

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn search_query(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<Track>, SignalError> {
    if query.trim().is_empty() {
        return Ok(Vec::new());
    }
    signal_search::search(&state.db, &query, SEARCH_LIMIT)
        .await
        .map_err(|e| match e {
            signal_search::SearchError::Execution(msg) => SignalError::Search(msg),
            other => SignalError::InvalidQuery {
                reason: other.to_string(),
            },
        })
}
