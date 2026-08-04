-- Remote OpenSubsonic servers Signal browses and streams from as a *client*
-- (docs/11-subsonic-client.md). Deliberately standalone: remote tracks get no
-- row in tracks/albums/artists, because tracks.file_path is NOT NULL UNIQUE and
-- every consumer relies on it naming a real file on disk.
CREATE TABLE remote_sources (
    id                 INTEGER PRIMARY KEY AUTOINCREMENT,
    name               TEXT NOT NULL,
    base_url           TEXT NOT NULL,
    username           TEXT NOT NULL,
    -- plaintext, matching the existing settings('server.password') precedent
    password           TEXT NOT NULL,
    -- which credential form this server actually accepted, so steady-state
    -- requests don't re-probe token-then-plain on every call
    auth_mode          TEXT NOT NULL DEFAULT 'token' CHECK (auth_mode IN ('token', 'legacy_p')),
    allow_insecure_tls INTEGER NOT NULL DEFAULT 0 CHECK (allow_insecure_tls IN (0, 1)),
    enabled            INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
    last_ping_at       TEXT,
    last_ping_ok       INTEGER CHECK (last_ping_ok IN (0, 1)),
    created_at         TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at         TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

-- sidebar entries are keyed by name; two sources sharing a label are unusable
CREATE UNIQUE INDEX idx_remote_sources_name ON remote_sources(name);
