-- Audio authenticity analysis results (doctor's fake hi-res detector).
-- One row per analyzed track; re-analysis upserts in place.
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
