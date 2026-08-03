//! Batch loop over analysis candidates. A small worker pool decodes files
//! concurrently (the work is CPU-bound), while persistence and progress
//! events stay on one consumer so DB writes and event order are sane.
//! Cancel latency stays at roughly one file per worker.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use signal_core::{EventBus, SignalEvent};
use signal_db::{AnalysisCandidate, DbPool};

use crate::{analyze_file, AnalysisResult, Verdict};

pub struct Analyzer {
    db: DbPool,
    events: EventBus,
}

fn worker_count() -> usize {
    std::thread::available_parallelism().map_or(2, |n| (n.get() / 2).clamp(2, 4))
}

impl Analyzer {
    #[must_use]
    pub fn new(db: DbPool, events: EventBus) -> Self {
        Self { db, events }
    }

    /// Analyzes every candidate, persisting verdicts as completions arrive.
    /// `cancel` is checked before each file and polled inside decode loops.
    pub async fn run(&self, candidates: Vec<AnalysisCandidate>, cancel: Arc<AtomicBool>) {
        let total = u32::try_from(candidates.len()).unwrap_or(u32::MAX);
        let candidates = Arc::new(candidates);
        let next = Arc::new(AtomicUsize::new(0));
        let (tx, mut rx) = tokio::sync::mpsc::channel::<(usize, AnalysisResult)>(8);

        for _ in 0..worker_count() {
            let candidates = Arc::clone(&candidates);
            let next = Arc::clone(&next);
            let cancel = Arc::clone(&cancel);
            let tx = tx.clone();
            tokio::spawn(async move {
                loop {
                    if cancel.load(Ordering::SeqCst) {
                        break;
                    }
                    let index = next.fetch_add(1, Ordering::SeqCst);
                    let Some(candidate) = candidates.get(index) else {
                        break;
                    };
                    let result = analyze_blocking(candidate, &cancel).await;
                    if tx.send((index, result)).await.is_err() {
                        break;
                    }
                }
            });
        }
        drop(tx);

        let mut processed = 0_u32;
        let mut analyzed = 0_u32;
        let mut flagged = 0_u32;
        let mut errors = 0_u32;

        while let Some((index, result)) = rx.recv().await {
            // in-flight results after a cancel are discarded, not persisted
            if cancel.load(Ordering::SeqCst) {
                continue;
            }
            let Some(candidate) = candidates.get(index) else {
                continue;
            };
            processed += 1;

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
                tracing::warn!(
                    track_id = candidate.track_id,
                    "analysis upsert failed: {err}"
                );
                errors += 1;
            }

            self.events.publish(SignalEvent::AnalysisProgress {
                processed,
                total,
                track_id: candidate.track_id,
                title: candidate.title.clone(),
                artist: candidate.artist_name.clone(),
                verdict: result.verdict.as_str().to_owned(),
                detail: result.detail,
            });
        }

        let cancelled = cancel.load(Ordering::SeqCst);
        tracing::info!(
            analyzed,
            flagged,
            errors,
            cancelled,
            "audio analysis finished"
        );
        self.events.publish(SignalEvent::AnalysisDone {
            analyzed,
            flagged,
            errors,
            cancelled,
        });
    }
}

async fn analyze_blocking(
    candidate: &AnalysisCandidate,
    cancel: &Arc<AtomicBool>,
) -> AnalysisResult {
    let path = std::path::PathBuf::from(&candidate.file_path);
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
