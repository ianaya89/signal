//! Filesystem watcher: notify + debouncer feeding incremental scans.

use std::path::PathBuf;
use std::time::Duration;

use notify::RecursiveMode;
use notify_debouncer_full::{new_debouncer, DebounceEventResult, DebouncedEvent};

use crate::scanner::Scanner;

const DEBOUNCE: Duration = Duration::from_secs(2);

/// Keeps the watcher alive; dropping it stops watching.
pub struct WatcherHandle {
    _debouncer: Box<dyn std::any::Any + Send>,
}

/// Watches `root` recursively. Debounced create/modify/rename/delete events
/// are split into changed vs removed paths and applied through the scanner
/// on the provided tokio runtime handle.
pub fn spawn_watcher(
    scanner: Scanner,
    root: &std::path::Path,
    runtime: tokio::runtime::Handle,
) -> notify::Result<WatcherHandle> {
    let mut debouncer = new_debouncer(DEBOUNCE, None, move |result: DebounceEventResult| {
        let events = match result {
            Ok(events) => events,
            Err(errors) => {
                for err in errors {
                    tracing::warn!("watch error: {err}");
                }
                return;
            }
        };

        let mut changed: Vec<PathBuf> = Vec::new();
        let mut removed: Vec<PathBuf> = Vec::new();
        for event in events {
            sort_event(&event, &mut changed, &mut removed);
        }
        if changed.is_empty() && removed.is_empty() {
            return;
        }

        tracing::debug!(
            changed = changed.len(),
            removed = removed.len(),
            "fs changes detected"
        );
        let scanner = scanner.clone();
        runtime.spawn(async move {
            if let Err(err) = scanner.apply_fs_changes(changed, removed).await {
                tracing::error!("incremental scan failed: {err}");
            }
        });
    })?;

    debouncer.watch(root, RecursiveMode::Recursive)?;
    tracing::info!(root = %root.display(), "filesystem watcher started");

    Ok(WatcherHandle {
        _debouncer: Box::new(debouncer),
    })
}

fn sort_event(event: &DebouncedEvent, changed: &mut Vec<PathBuf>, removed: &mut Vec<PathBuf>) {
    use notify::EventKind;

    match event.kind {
        EventKind::Create(_) | EventKind::Modify(_) => {
            for path in &event.paths {
                // renames arrive as Modify with both old+new paths; existence
                // decides which side each path is on
                if path.is_file() {
                    changed.push(path.clone());
                } else if !path.exists() {
                    removed.push(path.clone());
                }
            }
        }
        EventKind::Remove(_) => {
            removed.extend(event.paths.iter().cloned());
        }
        _ => {}
    }
}
