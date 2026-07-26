//! Library scanner: recursive walk, lofty tag extraction, artwork resolution.
//!
//! One-shot full scans land here in M1; notify-based incremental updates
//! come in M4 (`docs/07-roadmap.md`).

#![allow(clippy::missing_errors_doc)]

mod artwork;
mod scanner;
mod tags;
mod watcher;

pub use scanner::{Excludes, ScanReport, Scanner, ScannerError};
pub use watcher::{spawn_watcher, WatcherHandle};
