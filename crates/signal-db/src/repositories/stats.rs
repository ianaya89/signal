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
pub struct StatsOverview {
    pub total_plays: u32,
    pub total_ms_played: u64,
    pub distinct_tracks: u32,
    pub heatmap: Vec<DayCount>,
    pub top_artists: Vec<NameCount>,
    pub top_codecs: Vec<NameCount>,
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
        })
    }
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
