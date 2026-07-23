use std::collections::HashSet;
use std::path::{Path, PathBuf};

use signal_core::{EventBus, SignalEvent};
use signal_db::{DbPool, NewTrack};
use walkdir::WalkDir;

use crate::artwork;
use crate::tags;

#[derive(Debug, thiserror::Error)]
pub enum ScannerError {
    #[error("root is not a directory: {0}")]
    InvalidRoot(PathBuf),
    #[error("db: {0}")]
    Db(#[from] sqlx::Error),
    #[error("task join: {0}")]
    Join(#[from] tokio::task::JoinError),
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ScanReport {
    pub added: u32,
    pub updated: u32,
    pub removed: u32,
    pub skipped: u32,
    pub errors: u32,
}

pub struct Scanner {
    db: DbPool,
    events: EventBus,
    cache_dir: PathBuf,
}

impl Scanner {
    #[must_use]
    pub fn new(db: DbPool, events: EventBus, cache_dir: PathBuf) -> Self {
        Self {
            db,
            events,
            cache_dir,
        }
    }

    /// One-shot recursive scan. Emits `scanner:progress` per file and
    /// `scanner:done` at the end. A single bad file never aborts the scan.
    #[tracing::instrument(skip(self))]
    pub async fn scan_full(&self, root: PathBuf) -> Result<ScanReport, ScannerError> {
        if !root.is_dir() {
            return Err(ScannerError::InvalidRoot(root));
        }

        let (files, walk_errors) = collect_audio_files(&root);
        let total = u64::try_from(files.len()).unwrap_or_default();
        tracing::info!(total, walk_errors, root = %root.display(), "scan started");

        if files.is_empty() {
            let message = if walk_errors > 0 {
                format!(
                    "no audio files found in {} ({walk_errors} unreadable entries — likely a permissions problem; use the folder picker or grant access in System Settings → Privacy → Files & Folders)",
                    root.display()
                )
            } else {
                format!("no audio files found in {}", root.display())
            };
            self.events.publish(SignalEvent::ScannerError { message });
        }

        let mut report = ScanReport::default();
        report.errors += walk_errors;
        // albums whose artwork has been resolved during this scan
        let mut art_done: HashSet<i64> = HashSet::new();

        for (processed, path) in (0u64..).zip(files) {
            self.events.publish(SignalEvent::ScannerProgress {
                processed,
                total,
                current_path: path.display().to_string(),
            });

            match self.import_file(&path, &mut art_done).await {
                Ok(Imported::Added) => report.added += 1,
                Ok(Imported::Skipped) => report.skipped += 1,
                Err(err) => {
                    report.errors += 1;
                    tracing::warn!(path = %path.display(), "import failed: {err}");
                }
            }
        }

        self.events.publish(SignalEvent::ScannerDone {
            added: report.added,
            updated: report.updated,
            removed: report.removed,
            skipped: report.skipped,
            errors: report.errors,
        });
        tracing::info!(
            added = report.added,
            skipped = report.skipped,
            errors = report.errors,
            "scan finished"
        );
        Ok(report)
    }

    #[tracing::instrument(skip(self, art_done), fields(path = %path.display()))]
    async fn import_file(
        &self,
        path: &Path,
        art_done: &mut HashSet<i64>,
    ) -> Result<Imported, ImportError> {
        let path_str = path.to_string_lossy().into_owned();
        if self.db.tracks().id_by_path(&path_str).await?.is_some() {
            return Ok(Imported::Skipped);
        }

        let owned = path.to_path_buf();
        let extracted = tokio::task::spawn_blocking(move || tags::extract(&owned))
            .await
            .map_err(|e| ImportError::Extract(e.to_string()))?
            .map_err(|e| ImportError::Extract(e.to_string()))?;

        let artist_id = self.db.artists().get_or_create(&extracted.artist).await?;
        let album_id = match &extracted.album {
            Some(album) => Some(
                self.db
                    .albums()
                    .upsert(album, artist_id, extracted.year)
                    .await?,
            ),
            None => None,
        };

        self.db
            .tracks()
            .insert(&NewTrack {
                title: extracted.title.clone(),
                artist_id,
                album_id,
                track_no: extracted.track_no,
                disc_no: extracted.disc_no,
                year: extracted.year,
                duration_ms: extracted.duration_ms,
                genres: extracted.genres.clone(),
                technical: extracted.technical,
            })
            .await?;

        if let Some(album_id) = album_id {
            if art_done.insert(album_id) {
                self.resolve_artwork(album_id, path, extracted.embedded_art)
                    .await?;
            }
        }

        Ok(Imported::Added)
    }

    async fn resolve_artwork(
        &self,
        album_id: i64,
        track_path: &Path,
        embedded: Option<(Vec<u8>, &'static str)>,
    ) -> Result<(), ImportError> {
        if self.db.albums().artwork_path(album_id).await?.is_some() {
            return Ok(());
        }

        let resolved = if let Some((bytes, ext)) = embedded {
            let cache_dir = self.cache_dir.clone();
            tokio::task::spawn_blocking(move || {
                artwork::cache_embedded(&cache_dir, album_id, &bytes, ext)
            })
            .await
            .map_err(|e| ImportError::Extract(e.to_string()))?
            .ok()
        } else {
            artwork::find_folder_art(track_path)
        };

        if let Some(path) = resolved {
            self.db
                .albums()
                .set_artwork(album_id, &path.to_string_lossy())
                .await?;
        }
        Ok(())
    }
}

enum Imported {
    Added,
    Skipped,
}

#[derive(Debug, thiserror::Error)]
enum ImportError {
    #[error("db: {0}")]
    Db(#[from] sqlx::Error),
    #[error("extract: {0}")]
    Extract(String),
}

fn collect_audio_files(root: &Path) -> (Vec<PathBuf>, u32) {
    let mut errors = 0u32;
    let files = WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(|entry| match entry {
            Ok(e) if e.file_type().is_file() && tags::is_audio_file(e.path()) => {
                Some(e.into_path())
            }
            Ok(_) => None,
            Err(err) => {
                tracing::warn!("walk error: {err}");
                errors += 1;
                None
            }
        })
        .collect();
    (files, errors)
}
