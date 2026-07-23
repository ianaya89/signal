-- Artists and albums must dedupe case-insensitively, and albums group by
-- the ALBUM ARTIST tag (scanner change) instead of per-track artists.
-- This migration merges existing case-duplicates and rebuilds the unique
-- indexes with COLLATE NOCASE.

DROP INDEX idx_artists_name;
DROP INDEX idx_albums_artist_name;

-- ---- merge artists that differ only by case (keep lowest id) ----
CREATE TEMP TABLE artist_map AS
SELECT a.id AS dup_id,
       (SELECT MIN(a2.id) FROM artists a2 WHERE lower(a2.name) = lower(a.name)) AS keep_id
FROM artists a;

UPDATE tracks
   SET artist_id = (SELECT keep_id FROM artist_map WHERE dup_id = tracks.artist_id)
 WHERE artist_id IN (SELECT dup_id FROM artist_map WHERE dup_id <> keep_id);

UPDATE albums
   SET artist_id = (SELECT keep_id FROM artist_map WHERE dup_id = albums.artist_id)
 WHERE artist_id IN (SELECT dup_id FROM artist_map WHERE dup_id <> keep_id);

DELETE FROM artists
 WHERE id IN (SELECT dup_id FROM artist_map WHERE dup_id <> keep_id);

-- ---- merge albums now colliding on (artist_id, name) case-insensitively ----
CREATE TEMP TABLE album_map AS
SELECT al.id AS dup_id,
       (SELECT MIN(al2.id) FROM albums al2
         WHERE al2.artist_id = al.artist_id
           AND lower(al2.name) = lower(al.name)) AS keep_id
FROM albums al;

UPDATE tracks
   SET album_id = (SELECT keep_id FROM album_map WHERE dup_id = tracks.album_id)
 WHERE album_id IN (SELECT dup_id FROM album_map WHERE dup_id <> keep_id);

DELETE FROM albums
 WHERE id IN (SELECT dup_id FROM album_map WHERE dup_id <> keep_id);

DROP TABLE artist_map;
DROP TABLE album_map;

CREATE UNIQUE INDEX idx_artists_name ON artists(name COLLATE NOCASE);
CREATE UNIQUE INDEX idx_albums_artist_name ON albums(artist_id, name COLLATE NOCASE);
