use signal_core::{Track, TrackTechnical};
use sqlx::sqlite::SqlitePool;

use crate::row::track_from_row;

/// Insert payload produced by `signal-scanner`. Timestamps are always set by
/// `SQLite` defaults so the ISO-8601 text format stays uniform in the DB.
#[derive(Debug, Clone)]
pub struct NewTrack {
    pub title: String,
    pub artist_id: i64,
    pub album_id: Option<i64>,
    pub track_no: Option<u32>,
    pub disc_no: Option<u32>,
    pub year: Option<i32>,
    pub duration_ms: u64,
    pub genres: Vec<String>,
    pub technical: TrackTechnical,
}

/// Full-form metadata edit from the UI. Names are matched case-insensitively
/// against existing rows (find-or-create); an empty album detaches the track.
#[derive(Debug, Clone)]
pub struct TrackMetadataUpdate {
    pub title: String,
    pub artist_name: String,
    pub album_name: String,
    pub year: Option<i64>,
    pub track_no: Option<i64>,
    pub disc_no: Option<i64>,
    /// `None` leaves genres untouched; `Some("")` clears them.
    pub genre: Option<String>,
}

pub struct TrackRepo {
    pool: SqlitePool,
}

impl TrackRepo {
    #[must_use]
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Inserts the track and links its genres in one transaction. The FTS5
    /// index rows are maintained by triggers (see `migrations/0001_init.sql`).
    pub async fn insert(&self, new: &NewTrack) -> sqlx::Result<i64> {
        let mut tx = self.pool.begin().await?;

        let tech = &new.technical;
        let track_id: i64 = sqlx::query_scalar(
            "INSERT INTO tracks (
                title, artist_id, album_id, track_no, disc_no, year, duration_ms,
                codec, container, bitrate_kbps, bit_depth, sample_rate_hz, channels,
                replaygain_track_gain, replaygain_album_gain, peak, dr_score,
                encoder, file_path, file_size_bytes, md5
             ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7,
                ?8, ?9, ?10, ?11, ?12, ?13,
                ?14, ?15, ?16, ?17,
                ?18, ?19, ?20, ?21
             )
             RETURNING id",
        )
        .bind(&new.title)
        .bind(new.artist_id)
        .bind(new.album_id)
        .bind(new.track_no.map(i64::from))
        .bind(new.disc_no.map(i64::from))
        .bind(new.year)
        .bind(i64::try_from(new.duration_ms).unwrap_or_default())
        .bind(&tech.codec)
        .bind(&tech.container)
        .bind(i64::from(tech.bitrate_kbps))
        .bind(tech.bit_depth.map(i64::from))
        .bind(i64::from(tech.sample_rate_hz))
        .bind(i64::from(tech.channels))
        .bind(tech.replaygain_track_gain)
        .bind(tech.replaygain_album_gain)
        .bind(tech.peak)
        .bind(tech.dr_score)
        .bind(&tech.encoder)
        .bind(tech.file_path.to_string_lossy().into_owned())
        .bind(i64::try_from(tech.file_size_bytes).unwrap_or_default())
        .bind(&tech.md5)
        .fetch_one(&mut *tx)
        .await?;

        for genre in &new.genres {
            let genre_id: i64 = sqlx::query_scalar(
                "INSERT INTO genres (name) VALUES (?1)
                 ON CONFLICT(name) DO UPDATE SET name = excluded.name
                 RETURNING id",
            )
            .bind(genre)
            .fetch_one(&mut *tx)
            .await?;

            sqlx::query("INSERT OR IGNORE INTO track_genres (track_id, genre_id) VALUES (?1, ?2)")
                .bind(track_id)
                .bind(genre_id)
                .execute(&mut *tx)
                .await?;
        }

        tx.commit().await?;
        Ok(track_id)
    }

    pub async fn get(&self, id: i64) -> sqlx::Result<Option<Track>> {
        let row = sqlx::query("SELECT * FROM tracks WHERE id = ?1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        row.as_ref().map(track_from_row).transpose()
    }

    pub async fn id_by_path(&self, path: &str) -> sqlx::Result<Option<i64>> {
        sqlx::query_scalar("SELECT id FROM tracks WHERE file_path = ?1")
            .bind(path)
            .fetch_optional(&self.pool)
            .await
    }

    pub async fn list_by_genre(&self, genre_id: i64) -> sqlx::Result<Vec<Track>> {
        let rows = sqlx::query(
            "SELECT t.* FROM tracks t
             JOIN track_genres tg ON tg.track_id = t.id
             WHERE tg.genre_id = ?1
             ORDER BY t.artist_id, t.album_id, t.disc_no, t.track_no",
        )
        .bind(genre_id)
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(track_from_row).collect()
    }

    /// First genre per track (alphabetical when several) — batch lookup for
    /// server responses, same shape as the artist/album name maps.
    pub async fn genre_map(&self) -> sqlx::Result<Vec<(i64, String)>> {
        sqlx::query_as(
            "SELECT tg.track_id, MIN(g.name)
             FROM track_genres tg JOIN genres g ON g.id = tg.genre_id
             GROUP BY tg.track_id",
        )
        .fetch_all(&self.pool)
        .await
    }

    /// Uniform random sample; used by the `OpenSubsonic` `getRandomSongs`.
    pub async fn random(&self, limit: u32) -> sqlx::Result<Vec<Track>> {
        let rows = sqlx::query("SELECT * FROM tracks ORDER BY RANDOM() LIMIT ?1")
            .bind(limit)
            .fetch_all(&self.pool)
            .await?;
        rows.iter().map(track_from_row).collect()
    }

    /// Everything the user marked: ♥ favorites and ✦ 4-5 star ratings, the
    /// same signal `discover`'s rediscover shelf treats as "loved".
    pub async fn list_loved(&self) -> sqlx::Result<Vec<Track>> {
        let rows = sqlx::query(
            "SELECT * FROM tracks
             WHERE favorite = 1 OR rating >= 4
             ORDER BY favorite DESC, rating DESC, artist_id, album_id, disc_no, track_no",
        )
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(track_from_row).collect()
    }

    /// Tracks directly inside `dir` (not in subdirectories).
    pub async fn list_in_dir(&self, dir: &str) -> sqlx::Result<Vec<Track>> {
        let prefix = format!("{}/", dir.trim_end_matches('/'));
        let direct = format!("{prefix}%");
        let nested = format!("{prefix}%/%");
        let rows = sqlx::query(
            "SELECT * FROM tracks
             WHERE file_path LIKE ?1 AND file_path NOT LIKE ?2
             ORDER BY file_path",
        )
        .bind(direct)
        .bind(nested)
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(track_from_row).collect()
    }

    /// Number of tracks anywhere under `dir`.
    pub async fn count_under(&self, dir: &str) -> sqlx::Result<i64> {
        sqlx::query_scalar("SELECT COUNT(*) FROM tracks WHERE file_path LIKE ?1")
            .bind(format!("{}/%", dir.trim_end_matches('/')))
            .fetch_one(&self.pool)
            .await
    }

    pub async fn count(&self) -> sqlx::Result<i64> {
        sqlx::query_scalar("SELECT COUNT(*) FROM tracks")
            .fetch_one(&self.pool)
            .await
    }

    /// `rating` 0 clears.
    pub async fn set_rating(&self, id: i64, rating: u8) -> sqlx::Result<()> {
        sqlx::query("UPDATE tracks SET rating = ?2 WHERE id = ?1")
            .bind(id)
            .bind(i64::from(rating.min(5)))
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn set_favorite(&self, id: i64, favorite: bool) -> sqlx::Result<()> {
        sqlx::query("UPDATE tracks SET favorite = ?2 WHERE id = ?1")
            .bind(id)
            .bind(i64::from(favorite))
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Retitles a track; the FTS row follows via the update trigger.
    pub async fn rename(&self, id: i64, new_title: &str) -> sqlx::Result<()> {
        sqlx::query(
            "UPDATE tracks SET title = ?2,
                    modified_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
             WHERE id = ?1",
        )
        .bind(id)
        .bind(new_title)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// All linked genre names (edit-form prefill).
    pub async fn genres_of(&self, id: i64) -> sqlx::Result<Vec<String>> {
        sqlx::query_scalar(
            "SELECT g.name FROM track_genres tg
             JOIN genres g ON g.id = tg.genre_id
             WHERE tg.track_id = ?1
             ORDER BY g.name",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await
    }

    /// Applies a full metadata edit: artist/album re-pointed by name
    /// (find-or-create, case-insensitive), genres replaced, orphaned
    /// album/artist rows swept, FTS re-indexed — one transaction.
    #[allow(clippy::too_many_lines)] // linear sequence; splitting hides the tx flow
    pub async fn update_metadata(&self, id: i64, meta: &TrackMetadataUpdate) -> sqlx::Result<()> {
        let mut tx = self.pool.begin().await?;

        let old: Option<(i64, Option<i64>)> =
            sqlx::query_as("SELECT artist_id, album_id FROM tracks WHERE id = ?1")
                .bind(id)
                .fetch_optional(&mut *tx)
                .await?;
        let Some((old_artist_id, old_album_id)) = old else {
            return Ok(());
        };

        let artist_id: i64 =
            match sqlx::query_scalar("SELECT id FROM artists WHERE name = ?1 COLLATE NOCASE")
                .bind(&meta.artist_name)
                .fetch_optional(&mut *tx)
                .await?
            {
                Some(found) => found,
                None => {
                    sqlx::query_scalar("INSERT INTO artists (name) VALUES (?1) RETURNING id")
                        .bind(&meta.artist_name)
                        .fetch_one(&mut *tx)
                        .await?
                }
            };

        let album_name = meta.album_name.trim();
        let album_id: Option<i64> = if album_name.is_empty() {
            None
        } else {
            let found: Option<i64> = sqlx::query_scalar(
                "SELECT id FROM albums
                 WHERE artist_id = ?1 AND name = ?2 COLLATE NOCASE",
            )
            .bind(artist_id)
            .bind(album_name)
            .fetch_optional(&mut *tx)
            .await?;
            Some(match found {
                Some(existing) => existing,
                None => {
                    sqlx::query_scalar(
                        "INSERT INTO albums (name, artist_id, year) VALUES (?1, ?2, ?3)
                         RETURNING id",
                    )
                    .bind(album_name)
                    .bind(artist_id)
                    .bind(meta.year)
                    .fetch_one(&mut *tx)
                    .await?
                }
            })
        };

        sqlx::query(
            "UPDATE tracks SET title = ?2, artist_id = ?3, album_id = ?4,
                    year = ?5, track_no = ?6, disc_no = ?7,
                    modified_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
             WHERE id = ?1",
        )
        .bind(id)
        .bind(meta.title.trim())
        .bind(artist_id)
        .bind(album_id)
        .bind(meta.year)
        .bind(meta.track_no)
        .bind(meta.disc_no)
        .execute(&mut *tx)
        .await?;

        if let Some(genre) = &meta.genre {
            sqlx::query("DELETE FROM track_genres WHERE track_id = ?1")
                .bind(id)
                .execute(&mut *tx)
                .await?;
            // comma-separated list → one link per genre
            for genre in genre.split(',').map(str::trim).filter(|g| !g.is_empty()) {
                let genre_id: i64 = match sqlx::query_scalar(
                    "SELECT id FROM genres WHERE name = ?1 COLLATE NOCASE",
                )
                .bind(genre)
                .fetch_optional(&mut *tx)
                .await?
                {
                    Some(found) => found,
                    None => {
                        sqlx::query_scalar("INSERT INTO genres (name) VALUES (?1) RETURNING id")
                            .bind(genre)
                            .fetch_one(&mut *tx)
                            .await?
                    }
                };
                sqlx::query(
                    "INSERT OR IGNORE INTO track_genres (track_id, genre_id) VALUES (?1, ?2)",
                )
                .bind(id)
                .bind(genre_id)
                .execute(&mut *tx)
                .await?;
            }
        }

        if let Some(old_album) = old_album_id {
            if album_id != Some(old_album) {
                sqlx::query(
                    "DELETE FROM albums WHERE id = ?1
                     AND NOT EXISTS (SELECT 1 FROM tracks WHERE album_id = ?1)",
                )
                .bind(old_album)
                .execute(&mut *tx)
                .await?;
            }
        }
        if old_artist_id != artist_id {
            sqlx::query(
                "DELETE FROM artists WHERE id = ?1
                 AND NOT EXISTS (SELECT 1 FROM tracks WHERE artist_id = ?1)
                 AND NOT EXISTS (SELECT 1 FROM albums WHERE artist_id = ?1)",
            )
            .bind(old_artist_id)
            .execute(&mut *tx)
            .await?;
        }

        crate::row::refresh_fts_row(&mut tx, id).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Returns true when a row was actually deleted.
    pub async fn delete_by_path(&self, path: &str) -> sqlx::Result<bool> {
        let result = sqlx::query("DELETE FROM tracks WHERE file_path = ?1")
            .bind(path)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Removes every track under `dir` (database only — files stay), then
    /// sweeps newly-orphaned albums and artists. Returns rows removed.
    pub async fn delete_under_dir(&self, dir: &str) -> sqlx::Result<u32> {
        let mut tx = self.pool.begin().await?;
        let result = sqlx::query("DELETE FROM tracks WHERE file_path LIKE ?1")
            .bind(format!("{}/%", dir.trim_end_matches('/')))
            .execute(&mut *tx)
            .await?;
        sweep_orphans(&mut tx).await?;
        tx.commit().await?;
        Ok(u32::try_from(result.rows_affected()).unwrap_or(u32::MAX))
    }

    /// (id, `file_path`, md5) for every track — relink candidate matching.
    pub async fn list_paths_md5(&self) -> sqlx::Result<Vec<(i64, String, Option<String>)>> {
        sqlx::query_as("SELECT id, file_path, md5 FROM tracks")
            .fetch_all(&self.pool)
            .await
    }

    /// Re-links a moved file: the old row (dead path, carries stats and
    /// playlist/queue membership) adopts the new row's path; the freshly
    /// imported duplicate row is dropped. Play counts from the new row are
    /// folded in.
    pub async fn relink(&self, old_id: i64, new_id: i64) -> sqlx::Result<()> {
        let mut tx = self.pool.begin().await?;

        let moved: Option<(String, i64, i64)> = sqlx::query_as(
            "SELECT file_path, file_size_bytes, play_count FROM tracks WHERE id = ?1",
        )
        .bind(new_id)
        .fetch_optional(&mut *tx)
        .await?;
        let Some((new_path, new_size, new_plays)) = moved else {
            return Ok(());
        };

        // free the UNIQUE(file_path) before repointing the survivor
        sqlx::query("DELETE FROM tracks WHERE id = ?1")
            .bind(new_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "UPDATE tracks SET file_path = ?2, file_size_bytes = ?3,
                    play_count = play_count + ?4,
                    modified_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
             WHERE id = ?1",
        )
        .bind(old_id)
        .bind(new_path)
        .bind(new_size)
        .bind(new_plays)
        .execute(&mut *tx)
        .await?;

        sweep_orphans(&mut tx).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Merges a duplicate into `keep`: stats fold in, playlist membership
    /// and play history repoint, the duplicate row is deleted.
    pub async fn merge_into(&self, keep_id: i64, drop_id: i64) -> sqlx::Result<()> {
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            "UPDATE tracks SET
                play_count = play_count + (SELECT play_count FROM tracks WHERE id = ?2),
                skip_count = skip_count + (SELECT skip_count FROM tracks WHERE id = ?2),
                rating = MAX(rating, (SELECT rating FROM tracks WHERE id = ?2)),
                favorite = MAX(favorite, (SELECT favorite FROM tracks WHERE id = ?2)),
                last_played_at = MAX(COALESCE(last_played_at, ''),
                                     COALESCE((SELECT last_played_at FROM tracks WHERE id = ?2), '')),
                added_at = MIN(added_at, (SELECT added_at FROM tracks WHERE id = ?2))
             WHERE id = ?1",
        )
        .bind(keep_id)
        .bind(drop_id)
        .execute(&mut *tx)
        .await?;
        // '' can leak in when both sides were NULL
        sqlx::query(
            "UPDATE tracks SET last_played_at = NULL WHERE id = ?1 AND last_played_at = ''",
        )
        .bind(keep_id)
        .execute(&mut *tx)
        .await?;

        sqlx::query("UPDATE OR IGNORE playlist_tracks SET track_id = ?1 WHERE track_id = ?2")
            .bind(keep_id)
            .bind(drop_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE play_events SET track_id = ?1 WHERE track_id = ?2")
            .bind(keep_id)
            .bind(drop_id)
            .execute(&mut *tx)
            .await?;

        // cascades clean queue entries and leftover playlist rows
        sqlx::query("DELETE FROM tracks WHERE id = ?1")
            .bind(drop_id)
            .execute(&mut *tx)
            .await?;

        sweep_orphans(&mut tx).await?;
        tx.commit().await?;
        Ok(())
    }
}

/// Deletes albums with no tracks left, then artists with neither tracks
/// nor albums. Call after any bulk track removal.
pub async fn sweep_orphans(tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>) -> sqlx::Result<()> {
    sqlx::query(
        "DELETE FROM albums WHERE NOT EXISTS
            (SELECT 1 FROM tracks WHERE tracks.album_id = albums.id)",
    )
    .execute(&mut **tx)
    .await?;
    sqlx::query(
        "DELETE FROM artists WHERE NOT EXISTS
            (SELECT 1 FROM tracks WHERE tracks.artist_id = artists.id)
         AND NOT EXISTS
            (SELECT 1 FROM albums WHERE albums.artist_id = artists.id)",
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}
