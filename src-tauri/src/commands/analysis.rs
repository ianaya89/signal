//! Audio authenticity analysis: the doctor's fake hi-res detector.

use std::sync::atomic::Ordering;

use signal_core::SignalError;
use signal_db::{AnalysisSummary, FlaggedTrack};
use tauri::State;

use crate::commands::DbResultExt as _;
use crate::state::AppState;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisReport {
    pub running: bool,
    pub summary: AnalysisSummary,
    pub flagged: Vec<FlaggedTrack>,
}

/// Kicks off background spectral analysis of lossless tracks and returns the
/// candidate count immediately; progress arrives on `analysis:progress` /
/// `analysis:done`. `force` drops previous results and re-analyzes everything.
#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn analysis_start(state: State<'_, AppState>, force: bool) -> Result<u32, SignalError> {
    if state.analyzing.swap(true, Ordering::SeqCst) {
        return Err(SignalError::Analysis(
            "an analysis is already running".into(),
        ));
    }

    let candidates = async {
        if force {
            state.db.analysis().clear().await.db_err()?;
        }
        state.db.analysis().candidates(force).await.db_err()
    }
    .await;
    let candidates = match candidates {
        Ok(candidates) => candidates,
        Err(err) => {
            state.analyzing.store(false, Ordering::SeqCst);
            return Err(err);
        }
    };
    if candidates.is_empty() {
        state.analyzing.store(false, Ordering::SeqCst);
        return Ok(0);
    }
    state.analysis_cancel.store(false, Ordering::SeqCst);

    let total = u32::try_from(candidates.len()).unwrap_or(u32::MAX);
    let analyzer = signal_analysis::Analyzer::new(state.db.clone(), state.events.clone());
    let analyzing = state.analyzing.clone();
    let cancel = state.analysis_cancel.clone();
    tauri::async_runtime::spawn(async move {
        analyzer.run(candidates, cancel).await;
        analyzing.store(false, Ordering::SeqCst);
    });
    Ok(total)
}

/// Stops the running analysis; takes effect within the current file.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn analysis_cancel(state: State<'_, AppState>) {
    state.analysis_cancel.store(true, Ordering::SeqCst);
}

/// Stored results plus whether a run is live, so the doctor view can restore
/// an in-flight run on remount.
#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn analysis_report(state: State<'_, AppState>) -> Result<AnalysisReport, SignalError> {
    Ok(AnalysisReport {
        running: state.analyzing.load(Ordering::SeqCst),
        summary: state.db.analysis().summary().await.db_err()?,
        flagged: state.db.analysis().flagged().await.db_err()?,
    })
}
