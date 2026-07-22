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

-- prevents duplicate artist rows when the scanner re-imports the same tag text
CREATE UNIQUE INDEX idx_artists_name ON artists(name);

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

-- FK lookup: artist detail page renders "albums by this artist"
CREATE INDEX idx_albums_artist_id ON albums(artist_id);

-- prevents duplicate album rows when the scanner rescans an already-imported folder
CREATE UNIQUE INDEX idx_albums_artist_name ON albums(artist_id, name);

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
    skip_count             INTEGER NOT NULL DEFAULT 0
);

-- FK lookup: artist detail page track listing
CREATE INDEX idx_tracks_artist_id ON tracks(artist_id);

-- album view query — the single hottest read path in the app
CREATE INDEX idx_tracks_album_id ON tracks(album_id);

-- "Recently Added" smart playlist + default library sort order
CREATE INDEX idx_tracks_added_at ON tracks(added_at);

-- "Recently Played" sort and "Never Played" smart playlist (IS NULL scan)
CREATE INDEX idx_tracks_last_played_at ON tracks(last_played_at);

-- partial index: Favorites view filters a small subset of a large table
CREATE INDEX idx_tracks_favorite ON tracks(favorite) WHERE favorite = 1;

-- duplicate-file detection during rescans (content hash, independent of path)
CREATE INDEX idx_tracks_md5 ON tracks(md5);

CREATE TABLE track_genres (
    track_id INTEGER NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    genre_id INTEGER NOT NULL REFERENCES genres(id) ON DELETE CASCADE,
    PRIMARY KEY (track_id, genre_id)
);

-- reverse lookup: "all tracks tagged Jazz" for smart playlists and genre browsing
CREATE INDEX idx_track_genres_genre_id ON track_genres(genre_id);

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

-- reverse lookup: "remove this track from every playlist" on library delete
CREATE INDEX idx_playlist_tracks_track_id ON playlist_tracks(track_id);

-- enforces a single well-defined ordering per playlist, no position collisions
CREATE UNIQUE INDEX idx_playlist_tracks_position ON playlist_tracks(playlist_id, position);

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

-- queue order is read as ORDER BY position; uniqueness keeps ordering unambiguous
CREATE UNIQUE INDEX idx_queue_items_position ON queue_items(position);

-- ============================================================
-- Listening history (append-only; denormalized counters on tracks)
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

-- per-track stats aggregation
CREATE INDEX idx_play_events_track_id ON play_events(track_id);

-- heatmap and any date-range stats query — scans by date, not by track
CREATE INDEX idx_play_events_started_at ON play_events(started_at);

-- ============================================================
-- Key/value settings
-- ============================================================

CREATE TABLE settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- ============================================================
-- FTS5 search index (contentless; rowid == tracks.id)
-- ============================================================

CREATE VIRTUAL TABLE tracks_fts USING fts5(
    title,
    artist_name,
    album_name,
    genre,
    content='',
    contentless_delete=1,
    tokenize='porter unicode61 remove_diacritics 2'
);

-- Single-track changes are handled by triggers; cross-track fan-out
-- (artist/album rename) is handled in signal-db repository transactions.

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

-- Contentless-delete FTS5 tables reject UPDATE of a column subset, so genre
-- changes re-index the whole row (delete + reinsert) instead of UPDATE.
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
