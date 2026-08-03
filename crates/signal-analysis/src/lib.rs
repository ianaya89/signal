//! Offline audio authenticity analysis for lossless files.
//!
//! Decodes a few windows of a track, computes a Welch power spectrum, and
//! flags the classic frauds: hi-res files upsampled from CD rates, lossless
//! files transcoded from a lossy ancestor, and 24-bit containers carrying
//! 16-bit content. Verdicts land in the `track_analysis` table and surface
//! in the doctor view.

mod decode;
mod dsp;
mod runner;
mod verdict;

pub use runner::Analyzer;

use std::path::Path;
use std::sync::atomic::AtomicBool;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Clean,
    Upsampled,
    Transcode,
    PaddedBits,
    Unreadable,
    Skipped,
}

impl Verdict {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Clean => "clean",
            Self::Upsampled => "upsampled",
            Self::Transcode => "transcode",
            Self::PaddedBits => "padded_bits",
            Self::Unreadable => "unreadable",
            Self::Skipped => "skipped",
        }
    }

    /// Verdicts that belong in the doctor's suspicious list.
    #[must_use]
    pub fn is_flagged(self) -> bool {
        matches!(self, Self::Upsampled | Self::Transcode | Self::PaddedBits)
    }
}

#[derive(Debug, Clone)]
pub struct AnalysisResult {
    pub verdict: Verdict,
    pub cutoff_hz: Option<u32>,
    pub effective_bit_depth: Option<u8>,
    pub cliff_db: Option<f64>,
    /// 0.0–1.0; how strongly the evidence supports the verdict.
    pub confidence: f64,
    /// Human-readable evidence shown verbatim in the doctor view.
    pub detail: String,
}

impl AnalysisResult {
    fn skipped(detail: impl Into<String>) -> Self {
        Self {
            verdict: Verdict::Skipped,
            cutoff_hz: None,
            effective_bit_depth: None,
            cliff_db: None,
            confidence: 0.0,
            detail: detail.into(),
        }
    }

    fn unreadable(detail: impl Into<String>) -> Self {
        Self {
            verdict: Verdict::Unreadable,
            ..Self::skipped(detail)
        }
    }
}

const MIN_DURATION_MS: u64 = 5_000;

/// Analyzes one file. Never fails: undecodable input yields
/// [`Verdict::Unreadable`], sub-5s input [`Verdict::Skipped`]. `cancel` is
/// polled inside the decode loop; a cancelled run comes back as `Skipped`
/// (the runner discards it without persisting).
#[must_use]
pub fn analyze_file(
    path: &Path,
    claimed_bit_depth: Option<u8>,
    sample_rate_hz: u32,
    duration_ms: u64,
    cancel: &AtomicBool,
) -> AnalysisResult {
    if duration_ms < MIN_DURATION_MS {
        return AnalysisResult::skipped("shorter than 5s");
    }
    let decoded = match decode::decode_windows(path, duration_ms, cancel) {
        Ok(d) => d,
        Err(decode::DecodeFailure::Cancelled) => return AnalysisResult::skipped("cancelled"),
        Err(decode::DecodeFailure::Failed(msg)) => return AnalysisResult::unreadable(msg),
    };
    // The decoder's rate is authoritative; the DB value is a fallback for
    // containers that omit it.
    let sr = if decoded.sample_rate > 0 {
        decoded.sample_rate
    } else {
        sample_rate_hz
    };
    let Some(spectrum) = dsp::welch_psd(&decoded.mono_windows, sr) else {
        return AnalysisResult::unreadable("not enough decoded audio to analyze");
    };
    let metrics = dsp::spectral_metrics(&spectrum, sr);
    let effective_bits = decoded.int_stats.as_ref().and_then(dsp::effective_bits);
    verdict::classify(sr, claimed_bit_depth, &metrics, effective_bits)
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss
    )]

    use std::sync::atomic::{AtomicBool, Ordering};

    use super::*;
    use crate::dsp::tests::shaped_noise;

    const LEN: usize = 1 << 19;

    fn write_wav(dir: &tempfile::TempDir, name: &str, sample_rate: u32, to_i32: impl Fn(f32) -> i32) -> std::path::PathBuf {
        let path = dir.path().join(name);
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate,
            bits_per_sample: 24,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(&path, spec).unwrap();
        let f_max = if sample_rate >= 88_200 { 20_000.0 } else { 21_000.0 };
        for s in shaped_noise(LEN, sample_rate, move |f| if f <= f_max { 1.0 } else { 0.0 }) {
            writer.write_sample(to_i32(s)).unwrap();
        }
        writer.finalize().unwrap();
        path
    }

    fn duration_ms(sample_rate: u32) -> u64 {
        (LEN as u64 * 1_000) / u64::from(sample_rate)
    }

    #[test]
    fn upsampled_wav_is_flagged() {
        let dir = tempfile::tempdir().unwrap();
        // 96k container, content brickwalled at 20 kHz, honest 24-bit dither
        let path = write_wav(&dir, "fake96.wav", 96_000, |s| (s * 8_388_607.0) as i32);
        let cancel = AtomicBool::new(false);
        let r = analyze_file(&path, Some(24), 96_000, duration_ms(96_000), &cancel);
        assert_eq!(r.verdict, Verdict::Upsampled, "{}", r.detail);
        let cutoff = r.cutoff_hz.unwrap();
        assert!((19_000..=21_000).contains(&cutoff), "cutoff {cutoff}");
    }

    #[test]
    fn padded_24bit_wav_is_flagged() {
        let dir = tempfile::tempdir().unwrap();
        // full-band 44.1k content, but every sample is a 16-bit value << 8
        let path = write_wav(&dir, "padded.wav", 44_100, |s| ((s * 32_767.0) as i32) << 8);
        let cancel = AtomicBool::new(false);
        let r = analyze_file(&path, Some(24), 44_100, duration_ms(44_100), &cancel);
        assert_eq!(r.verdict, Verdict::PaddedBits, "{}", r.detail);
        assert_eq!(r.effective_bit_depth, Some(16));
    }

    #[test]
    fn genuine_24bit_wav_is_clean() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_wav(&dir, "real.wav", 44_100, |s| (s * 8_388_607.0) as i32);
        let cancel = AtomicBool::new(false);
        let r = analyze_file(&path, Some(24), 44_100, duration_ms(44_100), &cancel);
        assert_eq!(r.verdict, Verdict::Clean, "{}", r.detail);
        assert!(r.effective_bit_depth.unwrap() > 16);
    }

    #[test]
    fn garbage_file_is_unreadable() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("junk.flac");
        std::fs::write(&path, [0x13_u8, 0x37].repeat(512)).unwrap();
        let cancel = AtomicBool::new(false);
        let r = analyze_file(&path, Some(24), 96_000, 60_000, &cancel);
        assert_eq!(r.verdict, Verdict::Unreadable);
    }

    #[test]
    fn short_track_is_skipped_before_decoding() {
        let cancel = AtomicBool::new(false);
        let r = analyze_file(std::path::Path::new("/nonexistent"), None, 44_100, 3_000, &cancel);
        assert_eq!(r.verdict, Verdict::Skipped);
    }

    #[test]
    fn preset_cancel_flag_aborts_promptly() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_wav(&dir, "cancel.wav", 96_000, |s| (s * 8_388_607.0) as i32);
        let cancel = AtomicBool::new(true);
        let r = analyze_file(&path, Some(24), 96_000, duration_ms(96_000), &cancel);
        assert_eq!(r.verdict, Verdict::Skipped, "{}", r.detail);
        assert_eq!(r.detail, "cancelled");
        cancel.store(false, Ordering::Relaxed);
    }

    /// Regression lock on symphonia's FLAC left-shift behavior that the
    /// padded-bits math depends on. Soft-skips when ffmpeg is unavailable.
    #[test]
    fn flac_padded_vs_genuine_via_ffmpeg() {
        let dir = tempfile::tempdir().unwrap();
        let wav16 = dir.path().join("src16.wav");
        {
            let spec = hound::WavSpec {
                channels: 1,
                sample_rate: 44_100,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            };
            let mut w = hound::WavWriter::create(&wav16, spec).unwrap();
            for s in shaped_noise(LEN, 44_100, |f| if f <= 21_000.0 { 1.0 } else { 0.0 }) {
                w.write_sample((s * 32_767.0) as i16).unwrap();
            }
            w.finalize().unwrap();
        }
        let padded = dir.path().join("padded.flac");
        let status = std::process::Command::new("ffmpeg")
            .args(["-y", "-v", "error", "-i"])
            .arg(&wav16)
            .args(["-sample_fmt", "s32", "-bits_per_raw_sample", "24"])
            .arg(&padded)
            .status();
        let Ok(status) = status else {
            eprintln!("ffmpeg not found — skipping FLAC regression test");
            return;
        };
        assert!(status.success());

        let cancel = AtomicBool::new(false);
        let r = analyze_file(&padded, Some(24), 44_100, duration_ms(44_100), &cancel);
        assert_eq!(r.verdict, Verdict::PaddedBits, "{}", r.detail);
        assert_eq!(r.effective_bit_depth, Some(16));
    }
}
