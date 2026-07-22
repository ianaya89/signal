-- M0 placeholder so the migration pipeline exists before M1.
-- Full schema lands in M1 (docs/03-database-schema.md).
CREATE TABLE IF NOT EXISTS settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
) STRICT;
