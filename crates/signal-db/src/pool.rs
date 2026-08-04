use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions, SqliteSynchronous,
};

use crate::repositories::albums::AlbumRepo;
use crate::repositories::analysis::AnalysisRepo;
use crate::repositories::artists::ArtistRepo;
use crate::repositories::health::HealthRepo;
use crate::repositories::playlists::PlaylistRepo;
use crate::repositories::queue::QueueRepo;
use crate::repositories::remote_sources::RemoteSourceRepo;
use crate::repositories::settings::SettingsRepo;
use crate::repositories::stats::StatsRepo;
use crate::repositories::tracks::TrackRepo;

/// Cloneable handle over the connection pool; hands out repositories.
#[derive(Debug, Clone)]
pub struct DbPool {
    pool: SqlitePool,
}

impl DbPool {
    /// Opens (creating if missing) the database at `db_path`, applies WAL and
    /// FK pragmas on every pooled connection, and runs embedded migrations.
    pub async fn connect(db_path: &Path) -> sqlx::Result<Self> {
        let options = SqliteConnectOptions::from_str(&format!("sqlite://{}", db_path.display()))?
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .foreign_keys(true)
            .busy_timeout(Duration::from_secs(5))
            .pragma("temp_store", "memory");

        let pool = SqlitePoolOptions::new()
            .max_connections(8)
            .connect_with(options)
            .await?;

        crate::MIGRATOR.run(&pool).await?;
        tracing::info!(path = %db_path.display(), "database ready");

        Ok(Self { pool })
    }

    #[must_use]
    pub fn tracks(&self) -> TrackRepo {
        TrackRepo::new(self.pool.clone())
    }

    #[must_use]
    pub fn albums(&self) -> AlbumRepo {
        AlbumRepo::new(self.pool.clone())
    }

    #[must_use]
    pub fn artists(&self) -> ArtistRepo {
        ArtistRepo::new(self.pool.clone())
    }

    #[must_use]
    pub fn queue(&self) -> QueueRepo {
        QueueRepo::new(self.pool.clone())
    }

    #[must_use]
    pub fn playlists(&self) -> PlaylistRepo {
        PlaylistRepo::new(self.pool.clone())
    }

    #[must_use]
    pub fn health(&self) -> HealthRepo {
        HealthRepo::new(self.pool.clone())
    }

    #[must_use]
    pub fn analysis(&self) -> AnalysisRepo {
        AnalysisRepo::new(self.pool.clone())
    }

    #[must_use]
    pub fn stats(&self) -> StatsRepo {
        StatsRepo::new(self.pool.clone())
    }

    #[must_use]
    pub fn remote_sources(&self) -> RemoteSourceRepo {
        RemoteSourceRepo::new(self.pool.clone())
    }

    #[must_use]
    pub fn settings(&self) -> SettingsRepo {
        SettingsRepo::new(self.pool.clone())
    }

    #[must_use]
    pub fn inner(&self) -> &SqlitePool {
        &self.pool
    }

    /// Wipes all library data (tracks cascade into genres links, playlists,
    /// queue and play history). Settings survive.
    pub async fn reset_library(&self) -> sqlx::Result<()> {
        let mut tx = self.pool.begin().await?;
        for table in ["tracks", "albums", "artists", "genres", "playlists"] {
            sqlx::query(&format!("DELETE FROM {table}"))
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await
    }
}
