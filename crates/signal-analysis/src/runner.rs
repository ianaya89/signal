//! Batch loop over analysis candidates. Mirrors the scanner's shape:
//! sequential, one `spawn_blocking` per file, per-track progress events,
//! a done event at the end. Sequential keeps cancel latency at one file;
//! bounded 2–4-way parallelism is a clean follow-up if runtimes hurt.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use signal_core::{EventBus, SignalEvent};
use signal_db::{AnalysisCandidate, DbPool};

use crate::{analyze_file, AnalysisResult, Verdict};

pub struct Analyzer {
    db: DbPool,
    events: EventBus,
}

impl Analyzer {
    #[must_use]
    pub fn new(db: DbPool, events: EventBus) -> Self {
        Self { db, events }
    }

    /// Analyzes every candidate, persisting verdicts as it goes. `cancel` is
    /// checked between tracks and polled inside each file's decode loop.
    pub async fn run(&self, candidates: Vec<AnalysisCandidate>, cancel: Arc<AtomicBool>) {
        let total = u32::try_from(candidates.len()).unwrap_or(u32::MAX);
        let mut analyzed = 0_u32;
        let mut flagged = 0_u32;
        let mut errors = 0_u32;
        let mut cancelled = false;

        for (index, candidate) in candidates.into_iter().enumerate() {
            if cancel.load(Ordering::SeqCst) {
                cancelled = true;
                break;
            }

            let result = analyze_blocking(&candidate, &cancel).await;
            // a mid-file cancel comes back as Skipped("cancelled"); either way
            // the flag decides, and the in-flight result is not persisted
            if cancel.load(Ordering::SeqCst) {
                cancelled = true;
                break;
            }
            let processed = u32::try_from(index + 1).unwrap_or(u32::MAX);

            match result.verdict {
                Verdict::Unreadable => errors += 1,
                Verdict::Skipped => {}
                verdict => {
                    analyzed += 1;
                    if verdict.is_flagged() {
                        flagged += 1;
                    }
                }
            }

            let upsert = self
                .db
                .analysis()
                .upsert(
                    candidate.track_id,
                    result.verdict.as_str(),
                    result.cutoff_hz.map(i64::from),
                    result.effective_bit_depth.map(i64::from),
                    result.cliff_db,
                    result.confidence,
                    &result.detail,
                )
                .await;
            if let Err(err) = upsert {
                tracing::warn!(track_id = candidate.track_id, "analysis upsert failed: {err}");
                errors += 1;
            }

            self.events.publish(SignalEvent::AnalysisProgress {
                processed,
                total,
                track_id: candidate.track_id,
                title: candidate.title,
                artist: candidate.artist_name,
                verdict: result.verdict.as_str().to_owned(),
                detail: result.detail,
            });
        }

        tracing::info!(analyzed, flagged, errors, cancelled, "audio analysis finished");
        self.events.publish(SignalEvent::AnalysisDone {
            analyzed,
            flagged,
            errors,
            cancelled,
        });
    }
}

async fn analyze_blocking(candidate: &AnalysisCandidate, cancel: &Arc<AtomicBool>) -> AnalysisResult {
    let path = PathBuf::from(&candidate.file_path);
    let claimed = candidate.bit_depth;
    let sample_rate = candidate.sample_rate_hz;
    let duration_ms = candidate.duration_ms;
    let cancel = Arc::clone(cancel);
    tokio::task::spawn_blocking(move || {
        if !path.is_file() {
            return AnalysisResult::unreadable("file missing on disk");
        }
        analyze_file(&path, claimed, sample_rate, duration_ms, &cancel)
    })
    .await
    .unwrap_or_else(|err| AnalysisResult::unreadable(format!("analysis task failed: {err}")))
}
