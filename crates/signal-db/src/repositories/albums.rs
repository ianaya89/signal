use signal_core::{AlbumSummary, Track};
use sqlx::sqlite::SqlitePool;
use sqlx::Row;

use crate::row::{to_u32, track_from_row};

const SUMMARY_SELECT: &str = "SELECT al.id, al.name, al.artist_id, ar.name AS artist_name,
        al.year, al.artwork_path, COUNT(t.id) AS track_count
 FROM albums al
 JOIN artists ar ON ar.id = al.artist_id
 LEFT JOIN tracks t ON t.album_id = al.id";

pub struct AlbumRepo {
    pool: SqlitePool,
}

impl AlbumRepo {
    #[must_use]
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Case-insensitive upsert on (artist, name) — the unique index is NOCASE.
    pub async fn upsert(&self, name: &str, artist_id: i64, year: Option<i32>) -> sqlx::Result<i64> {
        let existing: Option<i64> = sqlx::query_scalar(
            "SELECT id FROM albums WHERE artist_id = ?1 AND name = ?2 COLLATE NOCASE",
        )
        .bind(artist_id)
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;
        if let Some(id) = existing {
            if let Some(year) = year {
                sqlx::query("UPDATE albums SET year = COALESCE(year, ?2) WHERE id = ?1")
                    .bind(id)
                    .bind(year)
                    .execute(&self.pool)
                    .await?;
            }
            return Ok(id);
        }
        sqlx::query_scalar(
            "INSERT INTO albums (name, artist_id, year) VALUES (?1, ?2, ?3) RETURNING id",
        )
        .bind(name)
        .bind(artist_id)
        .bind(year)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn list_by_artist(&self, artist_id: i64) -> sqlx::Result<Vec<AlbumSummary>> {
        let rows = sqlx::query(&format!(
            "{SUMMARY_SELECT}
             WHERE al.artist_id = ?1
             GROUP BY al.id
             HAVING track_count > 0
             ORDER BY al.year, al.name COLLATE NOCASE"
        ))
        .bind(artist_id)
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(summary_from_row).collect()
    }

    pub async fn list(&self) -> sqlx::Result<Vec<AlbumSummary>> {
        let rows = sqlx::query(&format!(
            "{SUMMARY_SELECT}
             GROUP BY al.id
             HAVING track_count > 0
             ORDER BY ar.name COLLATE NOCASE, al.year, al.name COLLATE NOCASE"
        ))
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(summary_from_row).collect()
    }

    pub async fn get(&self, id: i64) -> sqlx::Result<Option<AlbumSummary>> {
        let row = sqlx::query(&format!(
            "{SUMMARY_SELECT}
             WHERE al.id = ?1
             GROUP BY al.id"
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        row.as_ref().map(summary_from_row).transpose()
    }

    pub async fn tracks(&self, album_id: i64) -> sqlx::Result<Vec<Track>> {
        let rows = sqlx::query(
            "SELECT * FROM tracks
             WHERE album_id = ?1
             ORDER BY disc_no, track_no",
        )
        .bind(album_id)
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(track_from_row).collect()
    }

    /// Renames and re-indexes affected FTS rows (see `ArtistRepo::rename`).
    pub async fn rename(&self, id: i64, new_name: &str) -> sqlx::Result<()> {
        let mut tx = self.pool.begin().await?;

        sqlx::query("UPDATE albums SET name = ?2 WHERE id = ?1")
            .bind(id)
            .bind(new_name)
            .execute(&mut *tx)
            .await?;

        let track_ids: Vec<i64> = sqlx::query_scalar("SELECT id FROM tracks WHERE album_id = ?1")
            .bind(id)
            .fetch_all(&mut *tx)
            .await?;
        for track_id in track_ids {
            crate::row::refresh_fts_row(&mut tx, track_id).await?;
        }

        tx.commit().await
    }

    pub async fn set_artwork(&self, id: i64, path: &str) -> sqlx::Result<()> {
        sqlx::query("UPDATE albums SET artwork_path = ?2 WHERE id = ?1")
            .bind(id)
            .bind(path)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn artwork_path(&self, id: i64) -> sqlx::Result<Option<String>> {
        let row: Option<Option<String>> =
            sqlx::query_scalar("SELECT artwork_path FROM albums WHERE id = ?1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;
        Ok(row.flatten())
    }
}

fn summary_from_row(row: &sqlx::sqlite::SqliteRow) -> sqlx::Result<AlbumSummary> {
    Ok(AlbumSummary {
        id: row.try_get("id")?,
        name: row.try_get("name")?,
        artist_id: row.try_get("artist_id")?,
        artist_name: row.try_get("artist_name")?,
        year: row
            .try_get::<Option<i64>, _>("year")?
            .map(|y| i32::try_from(y).unwrap_or_default()),
        artwork_path: row.try_get("artwork_path")?,
        track_count: to_u32(row.try_get::<i64, _>("track_count")?),
    })
}
