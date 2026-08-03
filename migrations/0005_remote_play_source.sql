-- Widen play_events.source CHECK to admit 'remote' (OpenSubsonic scrobbles).
-- SQLite can't alter CHECK constraints: rebuild the table. play_events is a
-- leaf child table (FK out to tracks only), so a plain copy is safe.
CREATE TABLE play_events_new (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    track_id   INTEGER NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    started_at TEXT NOT NULL,
    ms_played  INTEGER NOT NULL,
    completed  INTEGER NOT NULL DEFAULT 0 CHECK (completed IN (0, 1)),
    skipped    INTEGER NOT NULL DEFAULT 0 CHECK (skipped IN (0, 1)),
    source     TEXT NOT NULL CHECK (source IN ('queue', 'playlist', 'album', 'search', 'remote'))
);

INSERT INTO play_events_new
SELECT id, track_id, started_at, ms_played, completed, skipped, source
FROM play_events;

DROP TABLE play_events;
ALTER TABLE play_events_new RENAME TO play_events;

-- per-track stats aggregation
CREATE INDEX idx_play_events_track_id ON play_events(track_id);

-- heatmap and any date-range stats query — scans by date, not by track
CREATE INDEX idx_play_events_started_at ON play_events(started_at);
