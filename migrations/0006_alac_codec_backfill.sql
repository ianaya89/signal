-- The scanner used to label every MP4 audio track as AAC; it now sniffs the
-- stsd sample entry and reports ALAC properly. Rescans skip files already in
-- the library, so existing rows need a backfill. bit_depth is the reliable
-- discriminator: lofty only reports it for lossless codecs — AAC rows always
-- have it NULL, ALAC rows always carry 16/24.
UPDATE tracks
SET codec = 'ALAC'
WHERE codec = 'AAC'
  AND container = 'MP4'
  AND bit_depth IS NOT NULL;
