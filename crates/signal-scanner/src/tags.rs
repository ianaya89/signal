use std::path::{Path, PathBuf};

use lofty::file::{AudioFile, FileType, TaggedFileExt};
use lofty::probe::Probe;
use lofty::tag::{Accessor, ItemKey};
use signal_core::TrackTechnical;

/// Everything extracted from one audio file, ready for DB insertion.
#[derive(Debug)]
pub struct Extracted {
    pub title: String,
    pub artist: String,
    /// ALBUMARTIST tag, falling back to `artist` — used to group albums so
    /// "feat. X" track credits don't fragment an album.
    pub album_artist: String,
    pub album: Option<String>,
    pub track_no: Option<u32>,
    pub disc_no: Option<u32>,
    pub year: Option<i32>,
    pub duration_ms: u64,
    pub genres: Vec<String>,
    pub technical: TrackTechnical,
    /// First embedded picture, if any: (bytes, file extension).
    pub embedded_art: Option<(Vec<u8>, &'static str)>,
}

pub const AUDIO_EXTENSIONS: &[&str] = &[
    "flac", "mp3", "m4a", "mp4", "aac", "wav", "aiff", "aif", "ogg", "opus",
];

pub fn is_audio_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| AUDIO_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()))
}

#[derive(Debug, thiserror::Error)]
pub enum ExtractError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("tag parse: {0}")]
    Lofty(#[from] lofty::error::LoftyError),
}

/// Fields written back to the audio file when tag write-back is enabled.
#[derive(Debug, Clone)]
pub struct WriteBack {
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub year: Option<u32>,
    pub track_no: Option<u32>,
    pub disc_no: Option<u32>,
    /// comma-separated list, empty clears
    pub genre: Option<String>,
}

/// Blocking (file IO + rewrite) — callers run it via `spawn_blocking`.
/// Updates the file's primary tag in place.
pub fn write_back(path: &Path, meta: &WriteBack) -> Result<(), ExtractError> {
    use lofty::config::WriteOptions;
    use lofty::file::AudioFile as _;
    use lofty::tag::Tag;

    let mut tagged = Probe::open(path)?.read()?;
    if tagged.primary_tag_mut().is_none() {
        let tag_type = tagged.primary_tag_type();
        tagged.insert_tag(Tag::new(tag_type));
    }
    let Some(tag) = tagged.primary_tag_mut() else {
        return Ok(());
    };

    tag.set_title(meta.title.clone());
    tag.set_artist(meta.artist.clone());
    match &meta.album {
        Some(album) => tag.set_album(album.clone()),
        None => tag.remove_album(),
    }
    match meta.year {
        Some(year) => tag.set_year(year),
        None => tag.remove_year(),
    }
    match meta.track_no {
        Some(n) => tag.set_track(n),
        None => tag.remove_track(),
    }
    match meta.disc_no {
        Some(n) => tag.set_disk(n),
        None => tag.remove_disk(),
    }
    match &meta.genre {
        Some(genre) if !genre.trim().is_empty() => tag.set_genre(genre.clone()),
        _ => tag.remove_genre(),
    }

    tagged.save_to_path(path, WriteOptions::default())?;
    Ok(())
}

/// Blocking (file IO + parse) — callers run it via `spawn_blocking`.
pub fn extract(path: &Path) -> Result<Extracted, ExtractError> {
    let file_size_bytes = std::fs::metadata(path)?.len();
    let tagged = Probe::open(path)?.read()?;

    let props = tagged.properties();
    let file_type = tagged.file_type();
    let tag = tagged.primary_tag().or_else(|| tagged.first_tag());

    let stem_title = || {
        path.file_stem().map_or_else(
            || "unknown".to_owned(),
            |s| s.to_string_lossy().into_owned(),
        )
    };

    let title = tag
        .and_then(Accessor::title)
        .map_or_else(stem_title, std::borrow::Cow::into_owned);
    let artist = tag
        .and_then(Accessor::artist)
        .map_or_else(|| "Unknown Artist".to_owned(), std::borrow::Cow::into_owned);
    let album_artist = tag
        .and_then(|t| t.get_string(&ItemKey::AlbumArtist))
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map_or_else(|| artist.clone(), ToOwned::to_owned);
    let (album, album_disc_no) = match tag.and_then(Accessor::album) {
        Some(raw) => {
            let (clean, disc) = normalize_album(&raw);
            (Some(clean), disc)
        }
        None => (None, None),
    };
    let genres = tag
        .and_then(Accessor::genre)
        .map(|g| {
            g.split([';', '/', ','])
                .map(|s| s.trim().to_owned())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default();

    let embedded_art = tag.and_then(|t| t.pictures().first()).map(|pic| {
        let ext = match pic.mime_type() {
            Some(lofty::picture::MimeType::Png) => "png",
            _ => "jpg",
        };
        (pic.data().to_vec(), ext)
    });

    let encoder = tag
        .and_then(|t| t.get_string(&ItemKey::EncoderSoftware))
        .map(ToOwned::to_owned);

    let replaygain_track_gain = tag
        .and_then(|t| t.get_string(&ItemKey::ReplayGainTrackGain))
        .and_then(parse_gain_db);
    let replaygain_album_gain = tag
        .and_then(|t| t.get_string(&ItemKey::ReplayGainAlbumGain))
        .and_then(parse_gain_db);
    let peak = tag
        .and_then(|t| t.get_string(&ItemKey::ReplayGainTrackPeak))
        .and_then(|s| s.trim().parse::<f64>().ok());

    Ok(Extracted {
        title,
        artist,
        album_artist,
        album,
        track_no: tag.and_then(Accessor::track),
        disc_no: tag.and_then(Accessor::disk).or(album_disc_no),
        year: tag
            .and_then(Accessor::year)
            .map(|y| i32::try_from(y).unwrap_or_default()),
        duration_ms: u64::try_from(props.duration().as_millis()).unwrap_or_default(),
        genres,
        technical: TrackTechnical {
            codec: codec_name(file_type),
            container: container_name(path, file_type),
            bitrate_kbps: props.audio_bitrate().unwrap_or_default(),
            bit_depth: props.bit_depth(),
            sample_rate_hz: props.sample_rate().unwrap_or_default(),
            channels: props.channels().unwrap_or_default(),
            replaygain_track_gain,
            replaygain_album_gain,
            peak,
            dr_score: None,
            encoder,
            file_path: PathBuf::from(path),
            file_size_bytes,
            md5: None,
        },
        embedded_art,
    })
}

/// Strips disc-number suffixes rippers embed in album names —
/// "Album (1)", "Album [2]", "Album (Disc 2)", "Album CD1", "Album - Disc 3" —
/// so multi-disc releases group as one album. The parsed number becomes the
/// track's `disc_no` fallback. Plain words like "Volumen 4" or "Vol III" are
/// left alone: a bare trailing number only counts inside (), [] or after an
/// explicit cd/disc/disk keyword.
fn normalize_album(raw: &str) -> (String, Option<u32>) {
    let trimmed = raw.trim();

    // "(N)" / "[N]" pure numeric suffix
    for (open, close) in [('(', ')'), ('[', ']')] {
        if let Some(rest) = trimmed.strip_suffix(close) {
            if let Some(idx) = rest.rfind(open) {
                let inner = rest[idx + 1..].trim();
                if let Ok(n) = inner.parse::<u32>() {
                    if (1..=20).contains(&n) {
                        return (rest[..idx].trim_end().to_owned(), Some(n));
                    }
                }
                // "(disc N)" / "[cd N]"
                if let Some(n) = parse_disc_keyword(inner) {
                    return (rest[..idx].trim_end().to_owned(), Some(n));
                }
            }
        }
    }

    // trailing "CD1" / "Disc 2" / "- Disk 3" without brackets: find the
    // keyword start, require a separator before it, and the remainder must
    // parse fully as keyword+number
    let lower = trimmed.to_ascii_lowercase();
    for kw in ["disc", "disk", "cd"] {
        if let Some(idx) = lower.rfind(kw) {
            let before_ok = idx > 0
                && trimmed[..idx].ends_with(|c: char| c.is_whitespace() || c == '-' || c == '–');
            if before_ok {
                if let Some(n) = parse_disc_keyword(&trimmed[idx..]) {
                    let clean = trimmed[..idx].trim_end_matches(['-', '–', ' ']).to_owned();
                    if !clean.is_empty() {
                        return (clean, Some(n));
                    }
                }
            }
        }
    }

    (trimmed.to_owned(), None)
}

/// "disc 2" / "cd1" / "disk 3" → 2/1/3. Anything else → None.
fn parse_disc_keyword(s: &str) -> Option<u32> {
    let lower = s.to_ascii_lowercase();
    let rest = ["disc", "disk", "cd"]
        .iter()
        .find_map(|kw| lower.strip_prefix(kw))?;
    let digits = rest.trim_start_matches([' ', '.', '#', '-']);
    let n = digits.parse::<u32>().ok()?;
    (1..=20).contains(&n).then_some(n)
}

/// "+1.23 dB" / "-4.5 dB" / bare number → dB value.
fn parse_gain_db(s: &str) -> Option<f64> {
    s.trim()
        .trim_end_matches("dB")
        .trim_end_matches("DB")
        .trim()
        .parse::<f64>()
        .ok()
}

fn codec_name(file_type: FileType) -> String {
    match file_type {
        FileType::Flac => "FLAC",
        FileType::Mpeg => "MP3",
        // AAC vs ALAC inside MP4 is refined at playback time (M2)
        FileType::Mp4 | FileType::Aac => "AAC",
        FileType::Opus => "Opus",
        FileType::Vorbis => "Vorbis",
        FileType::Wav => "PCM (WAV)",
        FileType::Aiff => "PCM (AIFF)",
        _ => "Unknown",
    }
    .to_owned()
}

fn container_name(path: &Path, file_type: FileType) -> String {
    match file_type {
        FileType::Flac => "FLAC".to_owned(),
        FileType::Mpeg => "MPEG".to_owned(),
        FileType::Mp4 => "MP4".to_owned(),
        FileType::Opus | FileType::Vorbis => "Ogg".to_owned(),
        FileType::Wav => "WAV".to_owned(),
        FileType::Aiff => "AIFF".to_owned(),
        _ => path.extension().map_or_else(
            || "Unknown".to_owned(),
            |e| e.to_string_lossy().to_uppercase(),
        ),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn gain_parsing() {
        assert!((parse_gain_db("-4.56 dB").unwrap() - -4.56).abs() < f64::EPSILON);
        assert!((parse_gain_db("+1.2dB").unwrap() - 1.2).abs() < f64::EPSILON);
        assert!(parse_gain_db("0.98").is_some());
        assert_eq!(parse_gain_db("garbage"), None);
    }

    #[test]
    fn album_disc_suffix_normalization() {
        assert_eq!(
            normalize_album("Random Access Memories (1)"),
            ("Random Access Memories".to_owned(), Some(1))
        );
        assert_eq!(
            normalize_album("Random Access Memories (2)"),
            ("Random Access Memories".to_owned(), Some(2))
        );
        assert_eq!(
            normalize_album("Aventine [Disc 2]"),
            ("Aventine".to_owned(), Some(2))
        );
        assert_eq!(
            normalize_album("Live in Paris CD1"),
            ("Live in Paris".to_owned(), Some(1))
        );
        assert_eq!(
            normalize_album("The Wall - Disc 2"),
            ("The Wall".to_owned(), Some(2))
        );
        // real names must survive untouched
        assert_eq!(normalize_album("Volumen 4"), ("Volumen 4".to_owned(), None));
        assert_eq!(normalize_album("Vol III"), ("Vol III".to_owned(), None));
        assert_eq!(
            normalize_album("Bocanada (1999)"),
            ("Bocanada (1999)".to_owned(), None)
        );
        assert_eq!(normalize_album("4"), ("4".to_owned(), None));
    }

    #[test]
    fn audio_extension_filter() {
        assert!(is_audio_file(Path::new("/a/b.FLAC")));
        assert!(is_audio_file(Path::new("/a/b.opus")));
        assert!(!is_audio_file(Path::new("/a/cover.jpg")));
        assert!(!is_audio_file(Path::new("/a/noext")));
    }
}
