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
    let album = tag
        .and_then(Accessor::album)
        .map(std::borrow::Cow::into_owned);
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
        disc_no: tag.and_then(Accessor::disk),
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
    fn audio_extension_filter() {
        assert!(is_audio_file(Path::new("/a/b.FLAC")));
        assert!(is_audio_file(Path::new("/a/b.opus")));
        assert!(!is_audio_file(Path::new("/a/cover.jpg")));
        assert!(!is_audio_file(Path::new("/a/noext")));
    }
}
