//! Symphonia-based window decoder: pulls a few short regions of a file as
//! mono f32 plus bit-usage stats for the padded-depth check. Playback never
//! touches this path (that stays libmpv's job).

use std::fs::File;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use symphonia::core::audio::{AudioBuffer, AudioBufferRef, Signal};
use symphonia::core::codecs::{Decoder, DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error;
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo};
use symphonia::core::io::{MediaSourceStream, MediaSourceStreamOptions};
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::sample::Sample;
use symphonia::core::units::Time;

use crate::dsp::IntSampleStats;

const WINDOW_SECS: u64 = 8;
/// Below this, one window from the start; otherwise three spread windows so
/// a quiet intro or outro can't dominate the spectrum.
const SHORT_TRACK_SECS: u64 = 30;
const WINDOW_POSITIONS: [f64; 3] = [0.10, 0.50, 0.90];
const CANCEL_POLL_PACKETS: u32 = 64;
const MAX_CONSECUTIVE_DECODE_ERRORS: u32 = 32;

pub(crate) enum DecodeFailure {
    Cancelled,
    Failed(String),
}

pub(crate) struct DecodedWindows {
    pub mono_windows: Vec<Vec<f32>>,
    /// 0 when the container omits it; the caller falls back to the DB value.
    pub sample_rate: u32,
    /// `None` for float sources — trailing zeros mean nothing there.
    pub int_stats: Option<IntSampleStats>,
}

struct Collector {
    window: Vec<f32>,
    target: usize,
    min_tz: u32,
    nonzero: u64,
    has_float: bool,
}

pub(crate) fn decode_windows(
    path: &Path,
    duration_ms: u64,
    cancel: &AtomicBool,
) -> Result<DecodedWindows, DecodeFailure> {
    let (mut format, mut decoder, track_id, sample_rate) = open(path)?;

    let dur_secs = duration_ms / 1_000;
    let starts: Vec<u64> = if dur_secs < SHORT_TRACK_SECS {
        vec![0]
    } else {
        #[allow(
            clippy::cast_precision_loss,
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss
        )]
        WINDOW_POSITIONS
            .iter()
            .map(|p| ((dur_secs as f64 * p) as u64).min(dur_secs.saturating_sub(WINDOW_SECS + 1)))
            .collect()
    };

    let window_len = usize::try_from(u64::from(sample_rate.max(8_000)) * WINDOW_SECS)
        .map_err(|_| DecodeFailure::Failed("absurd sample rate".into()))?;

    let mut collector = Collector {
        window: Vec::new(),
        target: window_len,
        min_tz: u32::MAX,
        nonzero: 0,
        has_float: false,
    };
    let mut windows: Vec<Vec<f32>> = Vec::with_capacity(starts.len());
    let mut last_error = String::new();

    for (i, &start) in starts.iter().enumerate() {
        if start > 0 {
            let target = SeekTo::Time {
                time: Time::from(start),
                track_id: Some(track_id),
            };
            match format.seek(SeekMode::Coarse, target) {
                Ok(_) => decoder.reset(),
                // an unseekable first window can still decode from the top;
                // later failures keep whatever windows we already have
                Err(_) if i == 0 => {}
                Err(err) => {
                    last_error = format!("seek failed: {err}");
                    break;
                }
            }
        }

        collector.window = Vec::with_capacity(window_len);
        let eof = match fill_window(
            format.as_mut(),
            decoder.as_mut(),
            track_id,
            &mut collector,
            cancel,
        ) {
            Ok(eof) => eof,
            Err(DecodeFailure::Cancelled) => return Err(DecodeFailure::Cancelled),
            Err(DecodeFailure::Failed(msg)) => {
                last_error = msg;
                break;
            }
        };
        if !collector.window.is_empty() {
            windows.push(std::mem::take(&mut collector.window));
        }
        if eof {
            break;
        }
    }

    if windows.is_empty() {
        return Err(DecodeFailure::Failed(if last_error.is_empty() {
            "no decodable audio".into()
        } else {
            last_error
        }));
    }

    let int_stats = (!collector.has_float).then_some(IntSampleStats {
        min_trailing_zeros: collector.min_tz.min(31),
        nonzero_samples: collector.nonzero,
    });
    Ok(DecodedWindows {
        mono_windows: windows,
        sample_rate,
        int_stats,
    })
}

type Opened = (Box<dyn FormatReader>, Box<dyn Decoder>, u32, u32);

fn open(path: &Path) -> Result<Opened, DecodeFailure> {
    let file =
        File::open(path).map_err(|err| DecodeFailure::Failed(format!("open failed: {err}")))?;
    let mss = MediaSourceStream::new(Box::new(file), MediaSourceStreamOptions::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }
    let format_opts = FormatOptions {
        enable_gapless: false,
        ..FormatOptions::default()
    };
    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &format_opts, &MetadataOptions::default())
        .map_err(|err| DecodeFailure::Failed(format!("unrecognized container: {err}")))?;
    let format = probed.format;

    let track = format
        .default_track()
        .filter(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .or_else(|| {
            format
                .tracks()
                .iter()
                .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        })
        .ok_or_else(|| DecodeFailure::Failed("no decodable track".into()))?;
    let track_id = track.id;
    let sample_rate = track.codec_params.sample_rate.unwrap_or(0);

    let decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|err| DecodeFailure::Failed(format!("unsupported codec: {err}")))?;

    Ok((format, decoder, track_id, sample_rate))
}

/// Decodes packets into `collector.window` until full or end of stream.
/// Returns `Ok(true)` on end of stream.
fn fill_window(
    format: &mut dyn FormatReader,
    decoder: &mut dyn Decoder,
    track_id: u32,
    collector: &mut Collector,
    cancel: &AtomicBool,
) -> Result<bool, DecodeFailure> {
    let mut packets_since_poll = 0_u32;
    let mut consecutive_errors = 0_u32;

    while collector.window.len() < collector.target {
        packets_since_poll += 1;
        if packets_since_poll >= CANCEL_POLL_PACKETS {
            packets_since_poll = 0;
            if cancel.load(Ordering::Relaxed) {
                return Err(DecodeFailure::Cancelled);
            }
        }

        let packet = match format.next_packet() {
            Ok(packet) => packet,
            // symphonia's normal end-of-stream signal
            Err(Error::IoError(err)) if err.kind() == std::io::ErrorKind::UnexpectedEof => {
                return Ok(true);
            }
            Err(Error::ResetRequired) => {
                decoder.reset();
                continue;
            }
            Err(err) => return Err(DecodeFailure::Failed(format!("read failed: {err}"))),
        };
        if packet.track_id() != track_id {
            continue;
        }

        match decoder.decode(&packet) {
            Ok(buffer) => {
                consecutive_errors = 0;
                fold(&buffer, collector);
            }
            Err(Error::DecodeError(_)) => {
                consecutive_errors += 1;
                if consecutive_errors >= MAX_CONSECUTIVE_DECODE_ERRORS {
                    return Err(DecodeFailure::Failed("too many corrupt packets".into()));
                }
            }
            Err(Error::ResetRequired) => decoder.reset(),
            Err(err) => return Err(DecodeFailure::Failed(format!("decode failed: {err}"))),
        }
    }
    Ok(false)
}

fn fold(buffer: &AudioBufferRef<'_>, collector: &mut Collector) {
    #[allow(clippy::cast_possible_wrap, clippy::cast_possible_truncation)]
    match buffer {
        AudioBufferRef::U8(buf) => {
            fold_int(buf.as_ref(), collector, |v| (i32::from(v) - 128) << 24);
        }
        AudioBufferRef::U16(buf) => {
            fold_int(buf.as_ref(), collector, |v| (i32::from(v) - 32_768) << 16);
        }
        AudioBufferRef::U24(buf) => fold_int(buf.as_ref(), collector, |v| {
            ((i64::from(v.inner()) - 8_388_608) as i32) << 8
        }),
        AudioBufferRef::U32(buf) => {
            fold_int(buf.as_ref(), collector, |v| (v ^ 0x8000_0000) as i32);
        }
        AudioBufferRef::S8(buf) => fold_int(buf.as_ref(), collector, |v| i32::from(v) << 24),
        AudioBufferRef::S16(buf) => fold_int(buf.as_ref(), collector, |v| i32::from(v) << 16),
        // FLAC/ALAC decode to left-aligned full-scale S32 already; S24 keeps
        // its payload in the low 24 bits, so both end up left-aligned here.
        AudioBufferRef::S24(buf) => fold_int(buf.as_ref(), collector, |v| v.inner() << 8),
        AudioBufferRef::S32(buf) => fold_int(buf.as_ref(), collector, |v| v),
        AudioBufferRef::F32(buf) => fold_float(buf.as_ref(), collector),
        AudioBufferRef::F64(buf) => fold_float(buf.as_ref(), collector),
    }
}

#[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
fn fold_int<T: Sample>(buf: &AudioBuffer<T>, collector: &mut Collector, to_i32: impl Fn(T) -> i32) {
    let channels = buf.spec().channels.count().max(1);
    for frame in 0..buf.frames() {
        if collector.window.len() >= collector.target {
            return;
        }
        let mut sum = 0.0_f64;
        for ch in 0..channels {
            let s = to_i32(buf.chan(ch)[frame]);
            sum += f64::from(s);
            if s != 0 {
                collector.min_tz = collector.min_tz.min(s.trailing_zeros());
                collector.nonzero += 1;
            }
        }
        collector
            .window
            .push((sum / channels as f64 / f64::from(1_u32 << 31)) as f32);
    }
}

#[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
fn fold_float<T: Sample + Into<f64>>(buf: &AudioBuffer<T>, collector: &mut Collector) {
    collector.has_float = true;
    let channels = buf.spec().channels.count().max(1);
    for frame in 0..buf.frames() {
        if collector.window.len() >= collector.target {
            return;
        }
        let sum: f64 = (0..channels).map(|ch| buf.chan(ch)[frame].into()).sum();
        collector.window.push((sum / channels as f64) as f32);
    }
}
