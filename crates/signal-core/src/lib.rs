//! Shared domain types, events, errors, and configuration for Signal.
//!
//! This crate depends on no other workspace crate; every other crate depends
//! on it. See `docs/02-workspace.md` for the dependency rules.

pub mod config;
pub mod error;
pub mod events;
pub mod models;

pub use config::AppConfig;
pub use error::SignalError;
pub use events::{EventBus, SignalEvent};
pub use models::{
    Album, AlbumDetail, AlbumSummary, Artist, ArtistSummary, AudioDevice, Genre, PlayEvent,
    PlaySource, PlaybackStatus, PlayerState, Playlist, QueueItem, SmartOp, SmartPlaylist,
    SmartRule, Track, TrackTechnical,
};
