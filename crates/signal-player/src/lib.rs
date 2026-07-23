//! Playback engine: safe async-friendly wrapper over libmpv.
//!
//! A dedicated thread owns the mpv handle; the public [`Player`] API sends
//! commands over a channel and never blocks. mpv events and property changes
//! are mapped onto [`signal_core::SignalEvent`] and published on the bus.
//! Design: `docs/04-player-libmpv.md`.

#![allow(clippy::missing_errors_doc)]

mod engine;
mod player;

pub use player::{Player, PlayerError};
