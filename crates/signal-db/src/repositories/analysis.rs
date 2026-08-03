//! Audio authenticity analysis results (`track_analysis`): candidates to
//! analyze, verdict upserts, and the doctor's flagged list.

use serde::Serialize;
use sqlx::sqlite::SqlitePool;
use sqlx::Row;

use crate::row::to_u32;

const FLAGGED_CAP: i64 = 500;
const FLAGGED_VERDICTS: &str = "('upsampled', 'transcode', 'padded_bits')";
const LOSSLESS_CODECS: &str = "('FLAC', 'ALAC', 'PCM (WAV)', 'PCM (AIFF)')";

/// A lossless track queued for spectral analysis. Artist comes pre-joined so
/// per-track progress events cost no extra queries.
#[derive(Debug, Clone)]
pub struct AnalysisCandidate {
    pub track_id: i64,
    pub title: String,
    pub artist_name: String,
    pub file_path: String,
    pub bit_depth: Option<u8>,
    pub sample_rate_hz: u32,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlaggedTrack {
    pub id: i64,
    pub title: String,
    pub artist_name: String,
    pub album_id: i64,
    pub verdict: String,
    pub confidence: f64,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisSummary {
    pub analyzed_total: u32,
    pub clean: u32,
    pub upsampled: u32,
    pub transcode: u32,
    pub padded_bits: u32,
    pub unreadable: u32,
    pub last_run_at: Option<String>,
}

pub struct AnalysisRepo {
    pool: SqlitePool,
}

impl AnalysisRepo {
    #[must_use]
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Lossless tracks to analyze; `force` re-includes already-analyzed ones.
    pub async fn candidates(&self, force: bool) -> sqlx::Result<Vec<AnalysisCandidate>> {
        let sql = format!(
            "SELECT t.id, t.title, ar.name AS artist_name, t.file_path,
                    t.bit_depth, t.sample_rate_hz, t.duration_ms
             FROM tracks t JOIN artists ar ON ar.id = t.artist_id
             WHERE t.codec IN {LOSSLESS_CODECS}
               AND (?1 OR t.id NOT IN (SELECT track_id FROM track_analysis))
             ORDER BY t.id"
        );
        let rows = sqlx::query(&sql).bind(force).fetch_all(&self.pool).await?;
        rows.iter()
            .map(|row| {
                Ok(AnalysisCandidate {
                    track_id: row.try_get("id")?,
                    title: row.try_get("title")?,
                    artist_name: row.try_get("artist_name")?,
                    file_path: row.try_get("file_path")?,
                    bit_depth: row
                        .try_get::<Option<i64>, _>("bit_depth")?
                        .and_then(|b| u8::try_from(b).ok()),
                    sample_rate_hz: to_u32(row.try_get("sample_rate_hz")?),
                    duration_ms: row
                        .try_get::<i64, _>("duration_ms")
                        .map(|ms| u64::try_from(ms).unwrap_or_default())?,
                })
            })
            .collect()
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn upsert(
        &self,
        track_id: i64,
        verdict: &str,
        cutoff_hz: Option<i64>,
        effective_bit_depth: Option<i64>,
        cliff_db: Option<f64>,
        confidence: f64,
        detail: &str,
    ) -> sqlx::Result<()> {
        sqlx::query(
            "INSERT INTO track_analysis
                 (track_id, verdict, cutoff_hz, effective_bit_depth, cliff_db, confidence, detail)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(track_id) DO UPDATE SET
                 analyzed_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                 verdict = excluded.verdict,
                 cutoff_hz = excluded.cutoff_hz,
                 effective_bit_depth = excluded.effective_bit_depth,
                 cliff_db = excluded.cliff_db,
                 confidence = excluded.confidence,
                 detail = excluded.detail",
        )
        .bind(track_id)
        .bind(verdict)
        .bind(cutoff_hz)
        .bind(effective_bit_depth)
        .bind(cliff_db)
        .bind(confidence)
        .bind(detail)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Suspicious tracks, strongest evidence first.
    pub async fn flagged(&self) -> sqlx::Result<Vec<FlaggedTrack>> {
        let sql = format!(
            "SELECT t.id, t.title, ar.name AS artist_name, t.album_id,
                    a.verdict, a.confidence, a.detail
             FROM track_analysis a
             JOIN tracks t ON t.id = a.track_id
             JOIN artists ar ON ar.id = t.artist_id
             WHERE a.verdict IN {FLAGGED_VERDICTS}
             ORDER BY a.confidence DESC, t.id
             LIMIT {FLAGGED_CAP}"
        );
        let rows = sqlx::query(&sql).fetch_all(&self.pool).await?;
        rows.iter()
            .map(|row| {
                Ok(FlaggedTrack {
                    id: row.try_get("id")?,
                    title: row.try_get("title")?,
                    artist_name: row.try_get("artist_name")?,
                    album_id: row.try_get("album_id")?,
                    verdict: row.try_get("verdict")?,
                    confidence: row.try_get("confidence")?,
                    detail: row.try_get("detail")?,
                })
            })
            .collect()
    }

    pub async fn summary(&self) -> sqlx::Result<AnalysisSummary> {
        let row = sqlx::query(
            "SELECT COUNT(*) AS total,
                    COALESCE(SUM(verdict = 'clean'), 0) AS clean,
                    COALESCE(SUM(verdict = 'upsampled'), 0) AS upsampled,
                    COALESCE(SUM(verdict = 'transcode'), 0) AS transcode,
                    COALESCE(SUM(verdict = 'padded_bits'), 0) AS padded_bits,
                    COALESCE(SUM(verdict = 'unreadable'), 0) AS unreadable,
                    MAX(analyzed_at) AS last_run_at
             FROM track_analysis",
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(AnalysisSummary {
            analyzed_total: to_u32(row.try_get("total")?),
            clean: to_u32(row.try_get("clean")?),
            upsampled: to_u32(row.try_get("upsampled")?),
            transcode: to_u32(row.try_get("transcode")?),
            padded_bits: to_u32(row.try_get("padded_bits")?),
            unreadable: to_u32(row.try_get("unreadable")?),
            last_run_at: row.try_get("last_run_at")?,
        })
    }

    /// Drops all results; used by force re-runs.
    pub async fn clear(&self) -> sqlx::Result<()> {
        sqlx::query("DELETE FROM track_analysis")
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
