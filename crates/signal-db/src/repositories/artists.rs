use signal_core::ArtistSummary;
use sqlx::sqlite::SqlitePool;
use sqlx::Row;

use crate::row::to_u32;

pub struct ArtistRepo {
    pool: SqlitePool,
}

impl ArtistRepo {
    #[must_use]
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Idempotent by the unique index on `artists.name`; the no-op
    /// `DO UPDATE` makes `RETURNING id` yield the existing row on conflict.
    pub async fn get_or_create(&self, name: &str) -> sqlx::Result<i64> {
        sqlx::query_scalar(
            "INSERT INTO artists (name) VALUES (?1)
             ON CONFLICT(name) DO UPDATE SET name = excluded.name
             RETURNING id",
        )
        .bind(name)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn list(&self) -> sqlx::Result<Vec<ArtistSummary>> {
        let rows = sqlx::query(
            "SELECT ar.id, ar.name,
                    COUNT(DISTINCT al.id) AS album_count,
                    COUNT(t.id) AS track_count
             FROM artists ar
             LEFT JOIN albums al ON al.artist_id = ar.id
             LEFT JOIN tracks t ON t.artist_id = ar.id
             GROUP BY ar.id
             HAVING track_count > 0
             ORDER BY ar.name COLLATE NOCASE",
        )
        .fetch_all(&self.pool)
        .await?;

        rows.iter()
            .map(|row| {
                Ok(ArtistSummary {
                    id: row.try_get("id")?,
                    name: row.try_get("name")?,
                    album_count: to_u32(row.try_get::<i64, _>("album_count")?),
                    track_count: to_u32(row.try_get::<i64, _>("track_count")?),
                })
            })
            .collect()
    }
}
