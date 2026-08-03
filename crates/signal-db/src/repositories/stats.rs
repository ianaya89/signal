use chrono::{DateTime, Utc};
use signal_core::PlaySource;
use sqlx::sqlite::SqlitePool;
use sqlx::Row;

use crate::row::to_u32;

#[derive(Debug, Clone)]
pub struct NewPlayEvent {
    pub track_id: i64,
    pub started_at: DateTime<Utc>,
    pub ms_played: u64,
    pub completed: bool,
    pub skipped: bool,
    pub source: PlaySource,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DayCount {
    pub day: String,
    pub plays: u32,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NameCount {
    pub name: String,
    pub count: u32,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumPlayCount {
    pub album_id: i64,
    pub name: String,
    pub artist_name: String,
    pub plays: u32,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackPlayCount {
    pub track_id: i64,
    pub album_id: i64,
    pub title: String,
    pub artist_name: String,
    pub plays: u32,
    pub favorite: bool,
    pub rating: u32,
}

/// Library shape, independent of listening history.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibrarySummary {
    pub tracks: u32,
    pub albums: u32,
    pub artists: u32,
    pub total_ms: u64,
    pub lossless_pct: u32,
    pub favorites: u32,
    pub liked: u32,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatsOverview {
    pub total_plays: u32,
    pub total_ms_played: u64,
    pub distinct_tracks: u32,
    pub heatmap: Vec<DayCount>,
    pub top_artists: Vec<NameCount>,
    pub top_codecs: Vec<NameCount>,
    pub top_albums: Vec<AlbumPlayCount>,
    pub top_tracks: Vec<TrackPlayCount>,
    /// plays per hour of day, index 0-23
    pub hourly: Vec<u32>,
    /// plays per weekday, index 0 = sunday
    pub weekday: Vec<u32>,
    pub library: LibrarySummary,
    /// consecutive days with at least one play, ending today or yesterday
    pub streak_current: u32,
    pub streak_best: u32,
}

pub struct StatsRepo {
    pool: SqlitePool,
}

impl StatsRepo {
    #[must_use]
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Appends the event and bumps the denormalized counters on `tracks`
    /// in one transaction so they can never drift from the event log.
    pub async fn log_play_event(&self, ev: &NewPlayEvent) -> sqlx::Result<i64> {
        let mut tx = self.pool.begin().await?;

        let source = match ev.source {
            PlaySource::Queue => "queue",
            PlaySource::Playlist => "playlist",
            PlaySource::Album => "album",
            PlaySource::Search => "search",
            PlaySource::Remote => "remote",
        };
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO play_events (track_id, started_at, ms_played, completed, skipped, source)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             RETURNING id",
        )
        .bind(ev.track_id)
        .bind(ev.started_at)
        .bind(i64::try_from(ev.ms_played).unwrap_or_default())
        .bind(i64::from(ev.completed))
        .bind(i64::from(ev.skipped))
        .bind(source)
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query(
            "UPDATE tracks SET
                play_count = play_count + ?2,
                skip_count = skip_count + ?3,
                last_played_at = ?4
             WHERE id = ?1",
        )
        .bind(ev.track_id)
        .bind(i64::from(!ev.skipped))
        .bind(i64::from(ev.skipped))
        .bind(ev.started_at)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(id)
    }

    // one linear sequence of independent aggregate queries; splitting adds nothing
    #[allow(clippy::too_many_lines)]
    pub async fn overview(&self, heatmap_days: u32) -> sqlx::Result<StatsOverview> {
        let totals = sqlx::query(
            "SELECT COUNT(*) AS plays, COALESCE(SUM(ms_played), 0) AS ms,
                    COUNT(DISTINCT track_id) AS tracks
             FROM play_events WHERE skipped = 0",
        )
        .fetch_one(&self.pool)
        .await?;

        let heatmap = sqlx::query(
            "SELECT date(started_at) AS day, COUNT(*) AS plays
             FROM play_events
             WHERE started_at >= date('now', ?1) AND skipped = 0
             GROUP BY day ORDER BY day",
        )
        .bind(format!("-{heatmap_days} days"))
        .fetch_all(&self.pool)
        .await?;

        let top_artists = sqlx::query(
            "SELECT ar.name AS name, COUNT(*) AS cnt
             FROM play_events pe
             JOIN tracks t ON t.id = pe.track_id
             JOIN artists ar ON ar.id = t.artist_id
             WHERE pe.skipped = 0
             GROUP BY ar.id ORDER BY cnt DESC LIMIT 10",
        )
        .fetch_all(&self.pool)
        .await?;

        let top_codecs = sqlx::query(
            "SELECT t.codec AS name, COUNT(*) AS cnt
             FROM play_events pe
             JOIN tracks t ON t.id = pe.track_id
             WHERE pe.skipped = 0
             GROUP BY t.codec ORDER BY cnt DESC LIMIT 10",
        )
        .fetch_all(&self.pool)
        .await?;

        let top_albums = sqlx::query(
            "SELECT al.id AS album_id, al.name AS name, ar.name AS artist_name,
                    COUNT(*) AS plays
             FROM play_events pe
             JOIN tracks t ON t.id = pe.track_id
             JOIN albums al ON al.id = t.album_id
             JOIN artists ar ON ar.id = al.artist_id
             WHERE pe.skipped = 0
             GROUP BY al.id ORDER BY plays DESC LIMIT 8",
        )
        .fetch_all(&self.pool)
        .await?;

        let top_tracks = sqlx::query(
            "SELECT t.id AS track_id, t.album_id AS album_id, t.title AS title,
                    ar.name AS artist_name, COUNT(*) AS plays,
                    t.favorite AS favorite, COALESCE(t.rating, 0) AS rating
             FROM play_events pe
             JOIN tracks t ON t.id = pe.track_id
             JOIN artists ar ON ar.id = t.artist_id
             WHERE pe.skipped = 0
             GROUP BY t.id ORDER BY plays DESC LIMIT 8",
        )
        .fetch_all(&self.pool)
        .await?;

        let hourly_rows = sqlx::query(
            "SELECT CAST(strftime('%H', started_at) AS INTEGER) AS hour, COUNT(*) AS cnt
             FROM play_events WHERE skipped = 0 GROUP BY hour",
        )
        .fetch_all(&self.pool)
        .await?;
        let mut hourly = vec![0u32; 24];
        for row in &hourly_rows {
            let hour: i64 = row.try_get("hour")?;
            if let Some(slot) = usize::try_from(hour).ok().and_then(|h| hourly.get_mut(h)) {
                *slot = to_u32(row.try_get::<i64, _>("cnt")?);
            }
        }

        let weekday_rows = sqlx::query(
            "SELECT CAST(strftime('%w', started_at) AS INTEGER) AS dow, COUNT(*) AS cnt
             FROM play_events WHERE skipped = 0 GROUP BY dow",
        )
        .fetch_all(&self.pool)
        .await?;
        let mut weekday = vec![0u32; 7];
        for row in &weekday_rows {
            let dow: i64 = row.try_get("dow")?;
            if let Some(slot) = usize::try_from(dow).ok().and_then(|d| weekday.get_mut(d)) {
                *slot = to_u32(row.try_get::<i64, _>("cnt")?);
            }
        }

        let library = sqlx::query(
            "SELECT COUNT(*) AS tracks,
                    COUNT(DISTINCT album_id) AS albums,
                    COUNT(DISTINCT artist_id) AS artists,
                    COALESCE(SUM(duration_ms), 0) AS total_ms,
                    COALESCE(SUM(codec IN ('FLAC','ALAC','PCM (WAV)','PCM (AIFF)')), 0) AS lossless,
                    COALESCE(SUM(favorite = 1), 0) AS favorites,
                    COALESCE(SUM(rating >= 4), 0) AS liked
             FROM tracks",
        )
        .fetch_one(&self.pool)
        .await?;
        let library_tracks = to_u32(library.try_get::<i64, _>("tracks")?);
        let lossless = to_u32(library.try_get::<i64, _>("lossless")?);

        let play_days: Vec<String> = sqlx::query_scalar(
            "SELECT DISTINCT date(started_at) FROM play_events
             WHERE skipped = 0 ORDER BY date(started_at)",
        )
        .fetch_all(&self.pool)
        .await?;
        let (streak_current, streak_best) = streaks(&play_days);

        Ok(StatsOverview {
            total_plays: to_u32(totals.try_get::<i64, _>("plays")?),
            total_ms_played: u64::try_from(totals.try_get::<i64, _>("ms")?).unwrap_or_default(),
            distinct_tracks: to_u32(totals.try_get::<i64, _>("tracks")?),
            heatmap: heatmap
                .iter()
                .map(|r| {
                    Ok(DayCount {
                        day: r.try_get("day")?,
                        plays: to_u32(r.try_get::<i64, _>("plays")?),
                    })
                })
                .collect::<sqlx::Result<_>>()?,
            top_artists: name_counts(&top_artists)?,
            top_codecs: name_counts(&top_codecs)?,
            top_albums: top_albums
                .iter()
                .map(|r| {
                    Ok(AlbumPlayCount {
                        album_id: r.try_get("album_id")?,
                        name: r.try_get("name")?,
                        artist_name: r.try_get("artist_name")?,
                        plays: to_u32(r.try_get::<i64, _>("plays")?),
                    })
                })
                .collect::<sqlx::Result<_>>()?,
            top_tracks: top_tracks
                .iter()
                .map(|r| {
                    Ok(TrackPlayCount {
                        track_id: r.try_get("track_id")?,
                        album_id: r.try_get("album_id")?,
                        title: r.try_get("title")?,
                        artist_name: r.try_get("artist_name")?,
                        plays: to_u32(r.try_get::<i64, _>("plays")?),
                        favorite: r.try_get::<i64, _>("favorite")? != 0,
                        rating: to_u32(r.try_get::<i64, _>("rating")?),
                    })
                })
                .collect::<sqlx::Result<_>>()?,
            hourly,
            weekday,
            library: LibrarySummary {
                tracks: library_tracks,
                albums: to_u32(library.try_get::<i64, _>("albums")?),
                artists: to_u32(library.try_get::<i64, _>("artists")?),
                total_ms: u64::try_from(library.try_get::<i64, _>("total_ms")?).unwrap_or_default(),
                lossless_pct: if library_tracks == 0 {
                    0
                } else {
                    u32::try_from(u64::from(lossless) * 100 / u64::from(library_tracks))
                        .unwrap_or_default()
                },
                favorites: to_u32(library.try_get::<i64, _>("favorites")?),
                liked: to_u32(library.try_get::<i64, _>("liked")?),
            },
            streak_current,
            streak_best,
        })
    }

    /// Recommendation shelves from listening signal — plain SQL, no ML.
    pub async fn discover(&self, per_shelf: i64) -> sqlx::Result<Discover> {
        let track_rows =
            |rows: Vec<sqlx::sqlite::SqliteRow>| -> sqlx::Result<Vec<signal_core::Track>> {
                rows.iter().map(crate::row::track_from_row).collect()
            };

        let on_repeat = track_rows(
            sqlx::query(
                "SELECT t.* FROM tracks t
                 JOIN (SELECT track_id, COUNT(*) AS cnt FROM play_events
                       WHERE started_at >= datetime('now', '-30 days')
                         AND skipped = 0
                       GROUP BY track_id) p ON p.track_id = t.id
                 ORDER BY p.cnt DESC
                 LIMIT ?1",
            )
            .bind(per_shelf)
            .fetch_all(&self.pool)
            .await?,
        )?;

        let rediscover = track_rows(
            sqlx::query(
                "SELECT * FROM tracks
                 WHERE (favorite = 1 OR rating >= 4)
                   AND (last_played_at IS NULL
                        OR last_played_at < datetime('now', '-30 days'))
                 ORDER BY rating DESC, play_count DESC
                 LIMIT ?1",
            )
            .bind(per_shelf)
            .fetch_all(&self.pool)
            .await?,
        )?;

        let from_your_artists = track_rows(
            sqlx::query(
                "SELECT * FROM tracks
                 WHERE play_count = 0
                   AND artist_id IN (
                       SELECT artist_id FROM tracks
                       GROUP BY artist_id
                       HAVING SUM(play_count) > 0
                       ORDER BY SUM(play_count) DESC
                       LIMIT 8)
                 ORDER BY RANDOM()
                 LIMIT ?1",
            )
            .bind(per_shelf)
            .fetch_all(&self.pool)
            .await?,
        )?;

        let never_played = track_rows(
            sqlx::query(
                "SELECT * FROM tracks
                 WHERE play_count = 0
                 ORDER BY RANDOM()
                 LIMIT ?1",
            )
            .bind(per_shelf)
            .fetch_all(&self.pool)
            .await?,
        )?;

        Ok(Discover {
            on_repeat,
            rediscover,
            from_your_artists,
            never_played,
        })
    }
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Discover {
    /// most played in the last 30 days
    pub on_repeat: Vec<signal_core::Track>,
    /// loved / rated but not heard lately
    pub rediscover: Vec<signal_core::Track>,
    /// unheard tracks by the artists you actually play
    pub from_your_artists: Vec<signal_core::Track>,
    /// random unplayed corners of the library
    pub never_played: Vec<signal_core::Track>,
}

/// (current, best) run of consecutive listening days. `days` is ascending
/// `YYYY-MM-DD`; the current run counts only if it reaches today or yesterday.
fn streaks(days: &[String]) -> (u32, u32) {
    use chrono::NaiveDate;

    let parsed: Vec<NaiveDate> = days
        .iter()
        .filter_map(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
        .collect();
    let Some(&first) = parsed.first() else {
        return (0, 0);
    };

    let mut best = 1u32;
    let mut run = 1u32;
    let mut prev = first;
    for &day in parsed.iter().skip(1) {
        run = if prev.succ_opt() == Some(day) {
            run + 1
        } else {
            1
        };
        best = best.max(run);
        prev = day;
    }

    let today = Utc::now().date_naive();
    let current = if prev == today || prev.succ_opt() == Some(today) {
        run
    } else {
        0
    };
    (current, best)
}

fn name_counts(rows: &[sqlx::sqlite::SqliteRow]) -> sqlx::Result<Vec<NameCount>> {
    rows.iter()
        .map(|r| {
            Ok(NameCount {
                name: r.try_get("name")?,
                count: to_u32(r.try_get::<i64, _>("cnt")?),
            })
        })
        .collect()
}
