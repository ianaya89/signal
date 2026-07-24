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

    /// Case-insensitive get-or-create (the unique index is NOCASE). The
    /// first-seen spelling wins; later case variants map onto it.
    pub async fn get_or_create(&self, name: &str) -> sqlx::Result<i64> {
        let existing: Option<i64> =
            sqlx::query_scalar("SELECT id FROM artists WHERE name = ?1 COLLATE NOCASE")
                .bind(name)
                .fetch_optional(&self.pool)
                .await?;
        if let Some(id) = existing {
            return Ok(id);
        }
        sqlx::query_scalar("INSERT INTO artists (name) VALUES (?1) RETURNING id")
            .bind(name)
            .fetch_one(&self.pool)
            .await
    }

    /// Album artists only — track-level "feat." credits don't clutter the
    /// artist list; their tracks are reachable through the album.
    pub async fn list(&self) -> sqlx::Result<Vec<ArtistSummary>> {
        let rows = sqlx::query(
            "SELECT ar.id, ar.name,
                    COUNT(DISTINCT al.id) AS album_count,
                    COUNT(t.id) AS track_count
             FROM artists ar
             JOIN albums al ON al.artist_id = ar.id
             LEFT JOIN tracks t ON t.album_id = al.id
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

    /// Renames and re-indexes every affected FTS row in one transaction.
    /// Contentless-delete FTS5 forbids partial UPDATE, so rows are
    /// delete+reinserted one by one (docs/03 §3).
    pub async fn rename(&self, id: i64, new_name: &str) -> sqlx::Result<()> {
        let mut tx = self.pool.begin().await?;

        sqlx::query("UPDATE artists SET name = ?2 WHERE id = ?1")
            .bind(id)
            .bind(new_name)
            .execute(&mut *tx)
            .await?;

        let track_ids: Vec<i64> = sqlx::query_scalar("SELECT id FROM tracks WHERE artist_id = ?1")
            .bind(id)
            .fetch_all(&mut *tx)
            .await?;

        for track_id in track_ids {
            crate::row::refresh_fts_row(&mut tx, track_id).await?;
        }

        tx.commit().await
    }

    pub async fn get(&self, id: i64) -> sqlx::Result<Option<ArtistSummary>> {
        let row = sqlx::query(
            "SELECT ar.id, ar.name,
                    COUNT(DISTINCT al.id) AS album_count,
                    COUNT(t.id) AS track_count
             FROM artists ar
             LEFT JOIN albums al ON al.artist_id = ar.id
             LEFT JOIN tracks t ON t.album_id = al.id
             WHERE ar.id = ?1
             GROUP BY ar.id",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|row| {
            Ok(ArtistSummary {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                album_count: to_u32(row.try_get::<i64, _>("album_count")?),
                track_count: to_u32(row.try_get::<i64, _>("track_count")?),
            })
        })
        .transpose()
    }
}
