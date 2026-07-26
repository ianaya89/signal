//! Database layer: sqlx `SqlitePool`, migrations, repositories.
//!
//! Repositories are the only sanctioned path to the database — no other
//! crate builds SQL against the pool. Design: `docs/03-database-schema.md`.

// All pub fns return sqlx::Result; the error is uniformly "database failure",
// documented once here instead of on every method.
#![allow(clippy::missing_errors_doc)]

mod pool;
mod repositories;
mod row;
pub mod smart;

pub use pool::DbPool;
pub use repositories::albums::AlbumRepo;
pub use repositories::artists::ArtistRepo;
pub use repositories::health::{HealthRepo, HealthReport};
pub use repositories::playlists::{PlaylistRepo, PlaylistSummary};
pub use repositories::queue::QueueRepo;
pub use repositories::settings::SettingsRepo;
pub use repositories::stats::{
    AlbumPlayCount, DayCount, Discover, NameCount, NewPlayEvent, StatsOverview, StatsRepo,
};
pub use repositories::tracks::{NewTrack, TrackMetadataUpdate, TrackRepo};
pub use row::track_from_row;

pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../migrations");
