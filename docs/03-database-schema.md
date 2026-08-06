# Database Schema

Signal stores its entire library, queue, playlists, and listening history in a single local SQLite database managed by `signal-db` via `sqlx`. This document specifies the full DDL, indexing rationale, the FTS5 search setup, the smart playlist rule format, connection configuration, and the repository API that the rest of the app (`src-tauri`, `signal-scanner`, `signal-player`) is built against.

## 1. Conventions

- **IDs**: every table uses `INTEGER PRIMARY KEY AUTOINCREMENT`, which aliases SQLite's `rowid`. This matters for `tracks_fts`, whose `rowid` is kept equal to `tracks.id` so search hits can be joined straight back without a mapping table.
- **Timestamps**: all `*_at` columns are `TEXT` storing ISO-8601 UTC (`YYYY-MM-DDTHH:MM:SS.SSSZ`). SQLite has no native datetime type; ISO-8601 strings sort and compare correctly as text and work directly with `date()`/`datetime()`/`strftime()` functions.
- **Booleans**: stored as `INTEGER` with a `CHECK (col IN (0,1))` constraint, since SQLite has no native `BOOLEAN` type.
- **Foreign keys**: `ON DELETE CASCADE` where the child row is meaningless without the parent (e.g. `track_genres`, `playlist_tracks`, `play_events`); `ON DELETE SET NULL` where the child should survive (e.g. `tracks.album_id` — a track can become "single/unknown album" if its album row is deleted without deleting the track itself).
- **JSON columns**: `smart_playlists.rules` is stored as `TEXT` containing serialized JSON (SQLite's `json_valid()` is used in a `CHECK` constraint), not the JSON1 binary format, so it round-trips cleanly through `serde_json` in Rust.

## 2. Full DDL — `migrations/0001_initial.sql`

```sql
PRAGMA foreign_keys = ON;

-- ============================================================
-- Core library tables
-- ============================================================

CREATE TABLE artists (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    name       TEXT NOT NULL,
    sort_name  TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE UNIQUE INDEX idx_artists_name ON artists(name);
-- prevents duplicate artist rows when the scanner re-imports the same tag text

CREATE TABLE genres (
    id   INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE
);

CREATE TABLE albums (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    name         TEXT NOT NULL,
    artist_id    INTEGER NOT NULL REFERENCES artists(id) ON DELETE CASCADE,
    year         INTEGER,
    artwork_path TEXT,
    added_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX idx_albums_artist_id ON albums(artist_id);
-- FK lookup: artist detail page renders "albums by this artist"

CREATE UNIQUE INDEX idx_albums_artist_name ON albums(artist_id, name);
-- prevents duplicate album rows when the scanner rescans an already-imported folder

CREATE TABLE tracks (
    id                     INTEGER PRIMARY KEY AUTOINCREMENT,
    title                  TEXT NOT NULL,
    artist_id              INTEGER NOT NULL REFERENCES artists(id) ON DELETE CASCADE,
    album_id               INTEGER REFERENCES albums(id) ON DELETE SET NULL,
    track_no               INTEGER,
    disc_no                INTEGER,
    year                   INTEGER,
    duration_ms            INTEGER NOT NULL,
    rating                 INTEGER NOT NULL DEFAULT 0 CHECK (rating BETWEEN 0 AND 5),
    favorite               INTEGER NOT NULL DEFAULT 0 CHECK (favorite IN (0, 1)),

    -- TrackTechnical (signal-core), populated by signal-scanner via lofty
    codec                  TEXT NOT NULL,
    container              TEXT NOT NULL,
    bitrate_kbps           INTEGER,
    bit_depth              INTEGER,
    sample_rate_hz         INTEGER NOT NULL,
    channels               INTEGER NOT NULL,
    replaygain_track_gain  REAL,
    replaygain_album_gain  REAL,
    peak                   REAL,
    dr_score               REAL,
    encoder                TEXT,
    file_path              TEXT NOT NULL UNIQUE,
    file_size_bytes        INTEGER NOT NULL,
    md5                    TEXT,

    added_at               TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    modified_at            TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    last_played_at         TEXT,
    play_count             INTEGER NOT NULL DEFAULT 0,
    skip_count              INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX idx_tracks_artist_id ON tracks(artist_id);
-- FK lookup: artist detail page track listing

CREATE INDEX idx_tracks_album_id ON tracks(album_id);
-- album view query (see §7) — the single hottest read path in the app

CREATE INDEX idx_tracks_added_at ON tracks(added_at);
-- "Recently Added" smart playlist + default library sort order

CREATE INDEX idx_tracks_last_played_at ON tracks(last_played_at);
-- "Recently Played" sort and "Never Played" smart playlist (IS NULL scan)

CREATE INDEX idx_tracks_favorite ON tracks(favorite) WHERE favorite = 1;
-- partial index: Favorites view filters a small subset of a large table

CREATE INDEX idx_tracks_md5 ON tracks(md5);
-- duplicate-file detection during rescans (content hash, independent of path)

CREATE TABLE track_genres (
    track_id INTEGER NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    genre_id INTEGER NOT NULL REFERENCES genres(id) ON DELETE CASCADE,
    PRIMARY KEY (track_id, genre_id)
);

CREATE INDEX idx_track_genres_genre_id ON track_genres(genre_id);
-- reverse lookup: "all tracks tagged Jazz" for smart playlists and genre browsing

-- ============================================================
-- Playlists (static + smart) — independent of the queue
-- ============================================================

CREATE TABLE playlists (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT NOT NULL,
    description TEXT,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE playlist_tracks (
    playlist_id INTEGER NOT NULL REFERENCES playlists(id) ON DELETE CASCADE,
    track_id    INTEGER NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    position    INTEGER NOT NULL,
    added_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    PRIMARY KEY (playlist_id, track_id)
);

CREATE INDEX idx_playlist_tracks_track_id ON playlist_tracks(track_id);
-- reverse lookup: "remove this track from every playlist" on library delete

CREATE UNIQUE INDEX idx_playlist_tracks_position ON playlist_tracks(playlist_id, position);
-- enforces a single well-defined ordering per playlist, no position collisions

CREATE TABLE smart_playlists (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    name       TEXT NOT NULL,
    rules      TEXT NOT NULL CHECK (json_valid(rules)),
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

-- ============================================================
-- Queue — git-staging-style, independent of playlists
-- ============================================================

CREATE TABLE queue_items (
    id       INTEGER PRIMARY KEY AUTOINCREMENT,
    position INTEGER NOT NULL,
    track_id INTEGER NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    added_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE UNIQUE INDEX idx_queue_items_position ON queue_items(position);
-- queue order is read as ORDER BY position; uniqueness keeps ordering unambiguous

-- ============================================================
-- Listening history
-- ============================================================

CREATE TABLE play_events (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    track_id   INTEGER NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    started_at TEXT NOT NULL,
    ms_played  INTEGER NOT NULL,
    completed  INTEGER NOT NULL DEFAULT 0 CHECK (completed IN (0, 1)),
    skipped    INTEGER NOT NULL DEFAULT 0 CHECK (skipped IN (0, 1)),
    source     TEXT NOT NULL CHECK (source IN ('queue', 'playlist', 'album', 'search'))
);

CREATE INDEX idx_play_events_track_id ON play_events(track_id);
-- per-track stats aggregation (play_count/skip_count backfill, "last played" audit)

CREATE INDEX idx_play_events_started_at ON play_events(started_at);
-- heatmap and any date-range stats query (see §7) — scans by date, not by track

-- ============================================================
-- Key/value settings
-- ============================================================

CREATE TABLE settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
```

`play_events` is intentionally append-only and never updated in place — `tracks.play_count`/`skip_count`/`last_played_at` are denormalized counters maintained alongside each insert (in the same transaction, see `StatsRepo::log_play_event` in §6) so hot-path reads (library list, now-playing) don't need a `JOIN`/`GROUP BY` against `play_events` just to show a play count.

## 3. FTS5 search — `tracks_fts`

Signal's search bar needs to match against `title`, `artist_name`, `album_name`, and `genre` in one query, but those four values live in four different normalized tables (`tracks`, `artists`, `albums`, `genres` via `track_genres`). SQLite's FTS5 "external content" mode only works when the FTS table's columns map 1:1 onto a *single* physical content table's columns by name — there is no such table here, since the schema is normalized and `tracks` itself has no `artist_name`/`album_name`/`genre` columns.

Rather than materializing a redundant denormalized table just to satisfy that constraint, `tracks_fts` is declared **contentless** (`content=''`): it stores only the inverted index, not a retrievable copy of the text. `rowid` is kept identical to `tracks.id`, so a `MATCH` query returns track IDs directly, which are then joined back against the normalized tables to render results. This is the "content=tracks-joined" strategy in practice: the source of truth for display is always `tracks`/`artists`/`albums`/`genres`; `tracks_fts` exists purely as a search index that is *populated from* a join at write time, never read back column-wise.

```sql
CREATE VIRTUAL TABLE tracks_fts USING fts5(
    title,
    artist_name,
    album_name,
    genre,
    content='',
    contentless_delete=1,
    tokenize='porter unicode61 remove_diacritics 2'
);
```

`contentless_delete=1` (SQLite ≥ 3.43) lets triggers delete a row by `rowid` alone (`DELETE FROM tracks_fts WHERE rowid = ?`) without needing to also supply the original column values, which a plain contentless table would otherwise require.

### Keeping it fresh

Direct changes to `tracks` (insert/delete/rename, or re-linking to a different artist/album) touch exactly one `tracks_fts` row, so they're handled with ordinary triggers:

```sql
CREATE TRIGGER trg_tracks_fts_ai
AFTER INSERT ON tracks
BEGIN
    INSERT INTO tracks_fts(rowid, title, artist_name, album_name, genre)
    SELECT
        new.id,
        new.title,
        (SELECT name FROM artists WHERE id = new.artist_id),
        (SELECT name FROM albums  WHERE id = new.album_id),
        (SELECT group_concat(g.name, ' ')
           FROM track_genres tg JOIN genres g ON g.id = tg.genre_id
          WHERE tg.track_id = new.id);
END;

CREATE TRIGGER trg_tracks_fts_ad
AFTER DELETE ON tracks
BEGIN
    DELETE FROM tracks_fts WHERE rowid = old.id;
END;

CREATE TRIGGER trg_tracks_fts_au
AFTER UPDATE OF title, artist_id, album_id ON tracks
BEGIN
    DELETE FROM tracks_fts WHERE rowid = old.id;
    INSERT INTO tracks_fts(rowid, title, artist_name, album_name, genre)
    SELECT
        new.id,
        new.title,
        (SELECT name FROM artists WHERE id = new.artist_id),
        (SELECT name FROM albums  WHERE id = new.album_id),
        (SELECT group_concat(g.name, ' ')
           FROM track_genres tg JOIN genres g ON g.id = tg.genre_id
          WHERE tg.track_id = new.id);
END;

-- NOTE: contentless-delete FTS5 tables reject `UPDATE` of a column subset
-- ("cannot UPDATE a subset of columns on fts5 contentless-delete table"),
-- so genre changes re-index the whole row (delete + reinsert):
CREATE TRIGGER trg_track_genres_ai
AFTER INSERT ON track_genres
BEGIN
    DELETE FROM tracks_fts WHERE rowid = new.track_id;
    INSERT INTO tracks_fts(rowid, title, artist_name, album_name, genre)
    SELECT
        t.id,
        t.title,
        (SELECT name FROM artists WHERE id = t.artist_id),
        (SELECT name FROM albums  WHERE id = t.album_id),
        (SELECT group_concat(g.name, ' ')
           FROM track_genres tg JOIN genres g ON g.id = tg.genre_id
          WHERE tg.track_id = t.id)
    FROM tracks t WHERE t.id = new.track_id;
END;

CREATE TRIGGER trg_track_genres_ad
AFTER DELETE ON track_genres
BEGIN
    DELETE FROM tracks_fts WHERE rowid = old.track_id;
    INSERT INTO tracks_fts(rowid, title, artist_name, album_name, genre)
    SELECT
        t.id,
        t.title,
        (SELECT name FROM artists WHERE id = t.artist_id),
        (SELECT name FROM albums  WHERE id = t.album_id),
        (SELECT group_concat(g.name, ' ')
           FROM track_genres tg JOIN genres g ON g.id = tg.genre_id
          WHERE tg.track_id = t.id)
    FROM tracks t WHERE t.id = old.track_id;
END;
```

Renaming an **artist** or **album**, however, potentially touches thousands of `tracks_fts` rows at once (every track by that artist). FTS5's virtual table implementation only reliably supports single-row `UPDATE ... WHERE rowid = ?`, not a correlated bulk `UPDATE ... WHERE rowid IN (subquery)`. Rather than fight that limitation with per-row triggers cascading across tables, this propagation is done at the application layer in `signal-db`, in the same transaction as the rename:

```rust
pub async fn rename_artist(pool: &SqlitePool, artist_id: i64, new_name: &str) -> sqlx::Result<()> {
    let mut tx = pool.begin().await?;

    sqlx::query!("UPDATE artists SET name = ? WHERE id = ?", new_name, artist_id)
        .execute(&mut *tx)
        .await?;

    let track_ids: Vec<i64> =
        sqlx::query_scalar!("SELECT id FROM tracks WHERE artist_id = ?", artist_id)
            .fetch_all(&mut *tx)
            .await?;

    for id in track_ids {
        sqlx::query!("UPDATE tracks_fts SET artist_name = ? WHERE rowid = ?", new_name, id)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await
}
```

`AlbumRepo::rename` follows the identical pattern for `album_name`. This keeps the index-maintenance rule simple: **single-track changes → SQL trigger; cross-track fan-out → repository method**, and it's the repository method's transaction boundary (not a trigger) that guarantees `tracks` and `tracks_fts` never observably diverge.

### Query shape

```sql
SELECT t.id, t.title, ar.name AS artist_name, al.name AS album_name
FROM tracks_fts f
JOIN tracks t   ON t.id = f.rowid
JOIN artists ar ON ar.id = t.artist_id
LEFT JOIN albums al ON al.id = t.album_id
WHERE tracks_fts MATCH ?1
ORDER BY rank
LIMIT 50;
```

## 4. Smart playlist rules

`smart_playlists.rules` is a JSON document with a fixed shape:

```jsonc
{
  "match": "all",           // "all" (AND) | "any" (OR)
  "conditions": [
    { "field": "play_count", "op": "eq", "value": 0 }
  ],
  "order_by": "added_at",   // any sortable field name below
  "order_dir": "desc",      // "asc" | "desc"
  "limit": null              // integer or null for unbounded
}
```

**Supported fields** (mapped to columns/joins by the compiler): `title`, `artist_name`, `album_name`, `genre`, `year`, `rating`, `favorite`, `play_count`, `skip_count`, `added_at`, `last_played_at`, `codec`, `bit_depth`, `sample_rate_hz`, `channels`, `duration_ms`.

**Supported operators**: `eq`, `neq`, `gt`, `gte`, `lt`, `lte`, `contains`, `not_contains`, `in`, `is_null`, `is_not_null`, `within_days` (relative-date convenience, e.g. `added_at within_days 30` for "Recently Added").

#### Example — Never Played

```json
{
  "match": "all",
  "conditions": [
    { "field": "play_count", "op": "eq", "value": 0 }
  ],
  "order_by": "added_at",
  "order_dir": "desc",
  "limit": null
}
```

#### Example — 24-bit only

```json
{
  "match": "all",
  "conditions": [
    { "field": "bit_depth", "op": "gte", "value": 24 }
  ],
  "order_by": "sample_rate_hz",
  "order_dir": "desc",
  "limit": null
}
```

#### Example — Jazz after 1990

```json
{
  "match": "all",
  "conditions": [
    { "field": "genre", "op": "contains", "value": "Jazz" },
    { "field": "year", "op": "gt", "value": 1990 }
  ],
  "order_by": "year",
  "order_dir": "asc",
  "limit": null
}
```

### Rule → SQL compilation

`signal-db::smart::compile(rules: &SmartPlaylistRules) -> (String, Vec<SqlValue>)` walks `conditions`, maps each `field` to either a direct column reference on `tracks`/`artists`/`albums`, or (for `genre`) an `EXISTS` subquery against `track_genres`, joins them with `AND`/`OR` per `match`, and returns a parameterized WHERE clause plus its bind values — parameters, never string interpolation, since rule values are user-authored via the UI.

The "Jazz after 1990" example compiles to:

```sql
SELECT t.* FROM tracks t
WHERE EXISTS (
    SELECT 1 FROM track_genres tg
    JOIN genres g ON g.id = tg.genre_id
    WHERE tg.track_id = t.id AND g.name = ?1
)
AND t.year > ?2
ORDER BY t.year ASC;
```
with bind params `["Jazz", 1990]`. The "Never Played" example compiles to a plain `WHERE t.play_count = 0 ORDER BY t.added_at DESC`, and "24-bit only" to `WHERE t.bit_depth >= 24 ORDER BY t.sample_rate_hz DESC`, with `artist_name`/`album_name` fields compiling to `ar.name`/`al.name` against joined `artists`/`albums` aliases the compiler adds automatically when referenced.

## 5. Connection setup

```rust
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use std::{path::Path, str::FromStr, time::Duration};

pub async fn connect(db_path: &Path) -> sqlx::Result<SqlitePool> {
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

    sqlx::migrate!("../migrations").run(&pool).await?;

    Ok(pool)
}
```

- **WAL mode**: readers (library list, search, stats views) never block on the single writer (scanner import, play-event logging), which matters because the scanner can hold long-running write transactions while the UI keeps querying.
- **`synchronous = NORMAL`**: safe under WAL (checkpoints are the durability boundary, not every commit) and meaningfully faster than `FULL` for the write-heavy scan phase.
- **`foreign_keys = ON`**: SQLite disables FK enforcement by default per-connection; `sqlx`'s `.foreign_keys(true)` sets the pragma on every pooled connection, not just the first, which is required since cascades (`ON DELETE CASCADE`) are load-bearing here (deleting a track must clean up `queue_items`, `playlist_tracks`, `play_events`, `track_genres`).
- **`busy_timeout`**: even under WAL, a writer can transiently block another writer; a 5s timeout absorbs that instead of surfacing `SQLITE_BUSY` to the UI during, e.g., a large batch insert from the scanner.

## 6. Repository layer

`signal-db` exposes one repository struct per aggregate, each holding a `SqlitePool` clone. All methods are `async fn ... -> sqlx::Result<T>`; callers (mostly Tauri commands and `signal-scanner`) own error mapping to their own error types.

```rust
pub struct TrackRepo { pool: SqlitePool }

impl TrackRepo {
    pub async fn insert(&self, new_track: NewTrack) -> sqlx::Result<Track>;
    pub async fn get(&self, id: i64) -> sqlx::Result<Option<Track>>;
    pub async fn find_by_path(&self, path: &str) -> sqlx::Result<Option<Track>>;
    pub async fn update_technical(&self, id: i64, tech: &TrackTechnical) -> sqlx::Result<()>;
    pub async fn set_rating(&self, id: i64, rating: u8) -> sqlx::Result<()>;
    pub async fn set_favorite(&self, id: i64, favorite: bool) -> sqlx::Result<()>;
    pub async fn delete(&self, id: i64) -> sqlx::Result<()>;
}

pub struct AlbumRepo { pool: SqlitePool }

impl AlbumRepo {
    pub async fn list(&self, sort: AlbumSort) -> sqlx::Result<Vec<Album>>;
    pub async fn get(&self, id: i64) -> sqlx::Result<Option<Album>>;
    pub async fn tracks(&self, album_id: i64) -> sqlx::Result<Vec<Track>>;
    pub async fn upsert(&self, name: &str, artist_id: i64, year: Option<i32>) -> sqlx::Result<i64>;
    pub async fn set_artwork(&self, id: i64, path: &str) -> sqlx::Result<()>;
    pub async fn rename(&self, id: i64, new_name: &str) -> sqlx::Result<()>;
}

pub struct PlaylistRepo { pool: SqlitePool }

impl PlaylistRepo {
    pub async fn create(&self, name: &str) -> sqlx::Result<i64>;
    pub async fn add_track(&self, playlist_id: i64, track_id: i64) -> sqlx::Result<()>;
    pub async fn remove_track(&self, playlist_id: i64, track_id: i64) -> sqlx::Result<()>;
    pub async fn reorder(&self, playlist_id: i64, ordered_track_ids: &[i64]) -> sqlx::Result<()>;
    pub async fn tracks(&self, playlist_id: i64) -> sqlx::Result<Vec<Track>>;
    pub async fn resolve_smart(&self, smart_playlist_id: i64) -> sqlx::Result<Vec<Track>>;
}

pub struct StatsRepo { pool: SqlitePool }

impl StatsRepo {
    pub async fn log_play_event(&self, ev: NewPlayEvent) -> sqlx::Result<i64>;
    pub async fn heatmap(&self, days: u32) -> sqlx::Result<Vec<DayCount>>;
    pub async fn top_genres(&self, limit: u32) -> sqlx::Result<Vec<GenreCount>>;
    pub async fn top_codecs(&self, limit: u32) -> sqlx::Result<Vec<CodecCount>>;
    pub async fn top_sample_rates(&self, limit: u32) -> sqlx::Result<Vec<SampleRateCount>>;
}

pub struct QueueRepo { pool: SqlitePool }

impl QueueRepo {
    pub async fn list(&self) -> sqlx::Result<Vec<QueueItem>>;
    pub async fn push_back(&self, track_id: i64) -> sqlx::Result<()>;
    pub async fn insert_next(&self, track_id: i64) -> sqlx::Result<()>;
    pub async fn remove(&self, queue_item_id: i64) -> sqlx::Result<()>;
    pub async fn reorder(&self, ordered_ids: &[i64]) -> sqlx::Result<()>;
    pub async fn clear(&self) -> sqlx::Result<()>;
}

pub struct RemoteSourceRepo { pool: SqlitePool }

impl RemoteSourceRepo {
    pub async fn list(&self) -> sqlx::Result<Vec<RemoteSource>>;
    pub async fn get(&self, id: i64) -> sqlx::Result<Option<RemoteSource>>;
    pub async fn credentials(&self, id: i64) -> sqlx::Result<Option<RemoteCredentials>>;
    pub async fn create(&self, name: &str, base_url: &str, username: &str, password: &str, allow_insecure_tls: bool) -> sqlx::Result<i64>;
    pub async fn update(&self, id: i64, patch: &RemoteSourcePatch) -> sqlx::Result<()>;
    pub async fn delete(&self, id: i64) -> sqlx::Result<()>;
    pub async fn record_ping(&self, id: i64, ok: bool, auth_mode: &str) -> sqlx::Result<()>;
}
```

`StatsRepo::log_play_event` is the one method that always runs as a transaction internally: it inserts into `play_events` and, in the same `tx`, bumps `tracks.play_count`/`skip_count` and sets `tracks.last_played_at`, so the denormalized counters on `tracks` can never drift from the append-only event log.

## 7. Typical queries

**Album view** (track listing for the album detail screen):

```sql
SELECT
    t.id, t.title, t.track_no, t.disc_no, t.duration_ms,
    t.codec, t.bit_depth, t.sample_rate_hz, t.bitrate_kbps,
    ar.name AS artist_name
FROM tracks t
JOIN artists ar ON ar.id = t.artist_id
WHERE t.album_id = ?1
ORDER BY t.disc_no, t.track_no;
```

**Stats heatmap** (plays per day, last 365 days — skipped plays excluded so partial listens don't inflate the map):

```sql
SELECT
    date(started_at) AS day,
    COUNT(*) AS play_count
FROM play_events
WHERE started_at >= date('now', '-365 days')
  AND skipped = 0
GROUP BY day
ORDER BY day;
```

**Top codecs** (library composition breakdown):

```sql
SELECT codec, COUNT(*) AS track_count
FROM tracks
GROUP BY codec
ORDER BY track_count DESC
LIMIT 10;
```

Both stats queries lean on `idx_play_events_started_at` and a full-table scan of `tracks` respectively — the codec/sample-rate breakdowns run over the whole library, so they're cheap enough (thousands, not millions, of rows) not to need a dedicated index; they're re-run on demand when the Stats view opens rather than cached.

## 8. Audio authenticity analysis — `track_analysis`

Added in `migrations/0004_track_analysis.sql`; written by `signal-analysis` via `AnalysisRepo`, read by the doctor view.

```sql
CREATE TABLE track_analysis (
    track_id            INTEGER PRIMARY KEY REFERENCES tracks(id) ON DELETE CASCADE,
    analyzed_at         TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    -- clean | upsampled | transcode | padded_bits | unreadable | skipped
    verdict             TEXT NOT NULL,
    cutoff_hz           INTEGER,
    effective_bit_depth INTEGER,
    cliff_db            REAL,
    confidence          REAL NOT NULL DEFAULT 0,
    detail              TEXT NOT NULL DEFAULT ''
);
CREATE INDEX idx_track_analysis_verdict ON track_analysis(verdict);
```

One row per analyzed track, upserted in place on re-analysis; `ON DELETE CASCADE` keeps it consistent with prunes. Only lossless codecs are ever analyzed (`codec IN ('FLAC', 'ALAC', 'PCM (WAV)', 'PCM (AIFF)')`).

Migration `0005_remote_play_source.sql` rebuilds `play_events` to widen the `source` CHECK with `'remote'` — plays scrobbled through the embedded OpenSubsonic server.

Migration `0006_alac_codec_backfill.sql` relabels existing ALAC rows the scanner had stored as `AAC` (rescans skip known paths, so new-scan sniffing alone can't fix them); `bit_depth IS NOT NULL` is the discriminator since lofty only reports bit depth for lossless codecs.

## 9. Remote sources — `remote_sources`

Added in `migrations/0007_remote_sources.sql`; written and read by `RemoteSourceRepo` in `crates/signal-db/src/repositories/remote_sources.rs`.

```sql
CREATE TABLE remote_sources (
    id                 INTEGER PRIMARY KEY AUTOINCREMENT,
    name               TEXT NOT NULL,
    base_url           TEXT NOT NULL,
    username           TEXT NOT NULL,
    password           TEXT NOT NULL,
    auth_mode          TEXT NOT NULL DEFAULT 'token' CHECK (auth_mode IN ('token', 'legacy_p')),
    allow_insecure_tls INTEGER NOT NULL DEFAULT 0 CHECK (allow_insecure_tls IN (0, 1)),
    enabled            INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
    last_ping_at       TEXT,
    last_ping_ok       INTEGER CHECK (last_ping_ok IN (0, 1)),
    created_at         TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at         TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE UNIQUE INDEX idx_remote_sources_name ON remote_sources(name);
```

This table is deliberately standalone: a remote track gets no row in `tracks`, `albums`, or `artists`. `tracks.file_path` is `NOT NULL UNIQUE` (§2) and every consumer of that column relies on it naming a real file on disk, so a track that lives on someone else's server cannot be represented there — `remote_sources` only ever describes the *server*, never its tracks. This is the single most important fact about this table, and everything else about how remote playback is wired follows from it.

The direct consequence is that a remote track also cannot be staged in `queue_items`, whose `track_id` column is `INTEGER NOT NULL REFERENCES tracks(id)` — there is no `tracks.id` to reference. Remote albums instead play through the in-memory play context rather than the persisted queue, which still gives auto-advance, gapless playback, and shuffle/repeat, just no `queue_items` rows.

`password` is stored as plaintext, matching the existing `settings('server.password')` precedent — it doesn't introduce a second, inconsistent secrets story for one table. `auth_mode` records which credential form the server actually accepted (`token` for salted-token auth, `legacy_p` for a plaintext `p=` query parameter), so steady-state requests don't have to re-probe token-then-plaintext on every call; it's written by the connection test, not chosen up front. `last_ping_at`/`last_ping_ok` are connection-test bookkeeping, surfaced in the settings UI as a per-server badge with a relative timestamp. The unique index on `name` exists because sidebar entries are keyed by name — two sources sharing a label would be unusable.

The repository, `RemoteSourceRepo`, exposes three distinct row shapes on purpose. `RemoteSource` omits `password` entirely — this is the shape listing returns over IPC, so the password has no path by which it could leak to the frontend. `RemoteCredentials` includes the password, and is read only when building a client. `RemoteSourcePatch` makes every field `Option`, applied via `COALESCE(?, col)` in a single `update()` statement rather than a read-modify-write, so an edit can't race another edit of the same row.

## 10. Migration strategy

Every schema change is a new, forward-only `.sql` file in `migrations/` at the repo root — no `.down.sql` files. A desktop app with a single embedded database can't meaningfully "roll back" a user's local schema mid-session anyway; a bad migration is fixed by shipping a corrective forward migration, not a revert.

```
migrations/
  0001_initial.sql
  0002_add_dr_score_index.sql
  0003_add_tags_table.sql
```

`sqlx-cli`'s `migrate add` defaults to timestamp-prefixed filenames; Signal renames to sequential `0001_`, `0002_`, ... prefixes instead, since a single-repo desktop schema doesn't need timestamp collision safety and sequential numbers are easier to scan in review.

```bash
cargo install sqlx-cli --no-default-features --features sqlite
sqlx migrate run --database-url sqlite://signal.db
sqlx migrate info
```

At build time, `signal-db` embeds every file under `migrations/` directly into the compiled binary:

```rust
pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../migrations");

pub async fn run_migrations(pool: &SqlitePool) -> sqlx::Result<()> {
    MIGRATOR.run(pool).await?;
    Ok(())
}
```

`sqlx::migrate!()` reads the directory at compile time (via `include_str!` under the hood), so the shipped executable carries its own migrations and needs no `migrations/` folder alongside it at runtime. `Migrator::run` creates and maintains sqlx's own `_sqlx_migrations` bookkeeping table, tracking applied version + a checksum of each file's contents — if a previously-applied migration file is edited after the fact, `run()` fails loudly on next launch instead of silently skipping or reapplying it.
