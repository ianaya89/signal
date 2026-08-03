//! Pure spectral math over decoded mono samples. No I/O — everything here
//! is deterministic and unit-tested against synthesized signals.

use realfft::RealFftPlanner;

pub(crate) const FFT_SIZE: usize = 8192;
const HOP: usize = FFT_SIZE / 2;
const SMOOTH_BINS: usize = 9;
const REF_BAND_HZ: (f64, f64) = (1_000.0, 8_000.0);
/// Bins this far below the 1–8 kHz median count as "no content".
const CONTENT_FLOOR_DB: f64 = 65.0;
/// Reference bands quieter than this are too close to digital silence to
/// tell a lowpass cliff from the noise floor.
const QUIET_REF_DB: f64 = -90.0;
/// Trailing-zero folds over fewer nonzero samples than this are not
/// trustworthy enough for a padded-bits verdict.
const MIN_EFFECTIVE_SAMPLES: u64 = 100_000;

pub(crate) struct Spectrum {
    /// Averaged Welch PSD in dB (arbitrary reference), DC..=Nyquist.
    pub psd_db: Vec<f64>,
    pub bin_hz: f64,
}

pub(crate) struct SpectralMetrics {
    pub too_quiet: bool,
    pub cutoff_hz: Option<u32>,
    /// Level drop from just below the cutoff to just above it. `None` when
    /// content runs so close to Nyquist there is nothing above to measure.
    pub cliff_db: Option<f64>,
}

/// Bit-usage stats folded over left-aligned i32 samples during decode.
pub(crate) struct IntSampleStats {
    pub min_trailing_zeros: u32,
    pub nonzero_samples: u64,
}

/// Welch-averaged PSD across all decoded windows: 8192-point Hann segments,
/// 50% overlap, f64 accumulation. `None` when no window holds a full segment.
#[allow(clippy::cast_precision_loss)]
pub(crate) fn welch_psd(windows: &[Vec<f32>], sample_rate: u32) -> Option<Spectrum> {
    let mut planner = RealFftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(FFT_SIZE);

    let hann: Vec<f32> = (0..FFT_SIZE)
        .map(|n| {
            let x = std::f64::consts::TAU * n as f64 / (FFT_SIZE - 1) as f64;
            #[allow(clippy::cast_possible_truncation)]
            let w = (0.5 - 0.5 * x.cos()) as f32;
            w
        })
        .collect();
    let window_power: f64 = hann.iter().map(|w| f64::from(*w) * f64::from(*w)).sum();

    let mut input = fft.make_input_vec();
    let mut output = fft.make_output_vec();
    let mut scratch = fft.make_scratch_vec();
    let mut acc = vec![0.0_f64; FFT_SIZE / 2 + 1];
    let mut segments = 0_u32;

    for samples in windows {
        let mut start = 0;
        while start + FFT_SIZE <= samples.len() {
            for (dst, (s, w)) in input.iter_mut().zip(samples[start..].iter().zip(&hann)) {
                *dst = s * w;
            }
            if fft
                .process_with_scratch(&mut input, &mut output, &mut scratch)
                .is_ok()
            {
                for (a, c) in acc.iter_mut().zip(&output) {
                    *a += f64::from(c.norm_sqr());
                }
                segments += 1;
            }
            start += HOP;
        }
    }
    if segments == 0 {
        return None;
    }

    let norm = f64::from(segments) * window_power;
    let psd_db = acc
        .iter()
        .map(|a| 10.0 * (a / norm + 1e-30).log10())
        .collect();
    Some(Spectrum {
        psd_db,
        bin_hz: f64::from(sample_rate) / FFT_SIZE as f64,
    })
}

/// Effective content bandwidth and cliff steepness, both relative to the
/// 1–8 kHz median so quiet masters measure the same as loud ones.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
pub(crate) fn spectral_metrics(spec: &Spectrum, sample_rate: u32) -> SpectralMetrics {
    let smoothed = smooth(&spec.psd_db);
    let last = smoothed.len() - 1;
    let bin_of = |hz: f64| -> usize { ((hz / spec.bin_hz).round() as usize).min(last) };

    let nyquist = f64::from(sample_rate) / 2.0;
    let ref_lo = bin_of(REF_BAND_HZ.0);
    let ref_hi = bin_of(REF_BAND_HZ.1.min(nyquist * 0.9)).max(ref_lo);
    let mut band = smoothed[ref_lo..=ref_hi].to_vec();
    band.sort_by(f64::total_cmp);
    let ref_db = band[band.len() / 2];

    if ref_db < QUIET_REF_DB {
        return SpectralMetrics {
            too_quiet: true,
            cutoff_hz: None,
            cliff_db: None,
        };
    }

    // Highest bin still holding content, sustained across 3 bins so an
    // isolated ultrasonic spur or dither spike can't fake a wide spectrum.
    let threshold = ref_db - CONTENT_FLOOR_DB;
    let cutoff_bin = (2..=last)
        .rev()
        .find(|&k| smoothed[k - 2..=k].iter().all(|&v| v >= threshold));

    let Some(cutoff_bin) = cutoff_bin else {
        return SpectralMetrics {
            too_quiet: false,
            cutoff_hz: None,
            cliff_db: None,
        };
    };
    let cutoff_hz = (cutoff_bin as f64 * spec.bin_hz).round() as u32;

    let bins_per = |hz: f64| -> usize { (hz / spec.bin_hz).round() as usize };
    let above_lo = cutoff_bin + bins_per(200.0);
    let above_hi = (cutoff_bin + bins_per(1_200.0)).min(last);
    let below_lo = cutoff_bin.saturating_sub(bins_per(1_000.0));
    let below_hi = cutoff_bin.saturating_sub(bins_per(200.0)).max(below_lo);
    let cliff_db = (above_hi.saturating_sub(above_lo) >= 20)
        .then(|| mean(&smoothed[below_lo..=below_hi]) - mean(&smoothed[above_lo..=above_hi]));

    SpectralMetrics {
        too_quiet: false,
        cutoff_hz: Some(cutoff_hz),
        cliff_db,
    }
}

/// `32 − min(trailing zeros)` over left-aligned nonzero samples; `None` when
/// the sample count is too small to trust (short or mostly-silent windows).
#[allow(clippy::cast_possible_truncation)]
pub(crate) fn effective_bits(stats: &IntSampleStats) -> Option<u8> {
    if stats.nonzero_samples < MIN_EFFECTIVE_SAMPLES {
        return None;
    }
    Some((32 - stats.min_trailing_zeros.min(31)) as u8)
}

fn smooth(psd: &[f64]) -> Vec<f64> {
    let half = SMOOTH_BINS / 2;
    let last = psd.len().saturating_sub(1);
    (0..psd.len())
        .map(|i| mean(&psd[i.saturating_sub(half)..=(i + half).min(last)]))
        .collect()
}

#[allow(clippy::cast_precision_loss)]
fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        return f64::NEG_INFINITY;
    }
    values.iter().sum::<f64>() / values.len() as f64
}

#[cfg(test)]
pub(crate) mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation
    )]

    use super::*;

    /// Deterministic xorshift — tests must not pull an RNG dependency.
    struct Rng(u64);
    impl Rng {
        fn next_f64(&mut self) -> f64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            (self.0 >> 11) as f64 / (1_u64 << 53) as f64
        }
    }

    /// Synthesizes noise whose spectrum follows `shape(freq_hz)` (linear
    /// magnitude), via random-phase inverse FFT. Peak-normalized to ~0.5.
    pub(crate) fn shaped_noise(
        len: usize,
        sample_rate: u32,
        shape: impl Fn(f64) -> f64,
    ) -> Vec<f32> {
        let mut planner = realfft::RealFftPlanner::<f64>::new();
        let ifft = planner.plan_fft_inverse(len);
        let mut spectrum = ifft.make_input_vec();
        let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
        let bin_hz = f64::from(sample_rate) / len as f64;
        for (k, c) in spectrum.iter_mut().enumerate() {
            let mag = shape(k as f64 * bin_hz);
            let phase = rng.next_f64() * std::f64::consts::TAU;
            *c = realfft::num_complex::Complex::from_polar(mag, phase);
        }
        spectrum[0] = 0.0.into();
        // realfft requires purely real DC and Nyquist bins
        if let Some(last) = spectrum.last_mut() {
            *last = last.norm().into();
        }
        let mut out = ifft.make_output_vec();
        ifft.process(&mut spectrum, &mut out).unwrap();
        let peak = out.iter().fold(0.0_f64, |m, s| m.max(s.abs())).max(1e-12);
        out.iter().map(|s| (s / peak * 0.5) as f32).collect()
    }

    const LEN: usize = 1 << 19; // ~5.5s at 96k, ~11.9s at 44.1k

    fn metrics_of(signal: Vec<f32>, sr: u32) -> SpectralMetrics {
        let spec = welch_psd(&[signal], sr).unwrap();
        spectral_metrics(&spec, sr)
    }

    #[test]
    fn detects_20khz_cutoff_in_96k_noise() {
        let m = metrics_of(
            shaped_noise(LEN, 96_000, |f| if f <= 20_000.0 { 1.0 } else { 0.0 }),
            96_000,
        );
        let cutoff = m.cutoff_hz.unwrap();
        assert!((19_500..=20_500).contains(&cutoff), "cutoff {cutoff}");
        assert!(m.cliff_db.unwrap() > 25.0);
        assert!(!m.too_quiet);
    }

    #[test]
    fn full_band_content_reads_to_nyquist() {
        let m = metrics_of(
            shaped_noise(LEN, 96_000, |f| if f <= 46_000.0 { 1.0 } else { 0.0 }),
            96_000,
        );
        assert!(m.cutoff_hz.unwrap() >= 44_000);
    }

    #[test]
    fn mp3_style_cliff_at_16k_with_noise_floor() {
        // -80 dB shelf above the cutoff mimics a lossy codec's leftover floor
        let m = metrics_of(
            shaped_noise(LEN, 44_100, |f| if f <= 16_000.0 { 1.0 } else { 1e-4 }),
            44_100,
        );
        let cutoff = m.cutoff_hz.unwrap();
        assert!((15_500..=16_500).contains(&cutoff), "cutoff {cutoff}");
        assert!(m.cliff_db.unwrap() >= 25.0);
    }

    #[test]
    fn gentle_analog_rolloff_has_no_cliff() {
        // -3 dB per kHz beyond 12 kHz: audible rolloff, but nothing like a
        // brickwall. Steepness across any 1 kHz span stays near 3 dB.
        let m = metrics_of(
            shaped_noise(LEN, 44_100, |f| {
                if f <= 12_000.0 {
                    1.0
                } else {
                    10_f64.powf(-((f - 12_000.0) / 1_000.0 * 3.0) / 20.0)
                }
            }),
            44_100,
        );
        if let Some(cliff) = m.cliff_db {
            assert!(cliff < 25.0, "gentle rolloff misread as cliff: {cliff} dB");
        }
    }

    #[test]
    fn near_silence_is_too_quiet_to_judge() {
        let quiet: Vec<f32> = shaped_noise(LEN, 44_100, |_| 1.0)
            .into_iter()
            .map(|s| s * 1e-6)
            .collect();
        let m = metrics_of(quiet, 44_100);
        assert!(m.too_quiet);
        assert!(m.cutoff_hz.is_none());
    }

    #[test]
    fn welch_needs_at_least_one_full_segment() {
        assert!(welch_psd(&[vec![0.0; FFT_SIZE - 1]], 44_100).is_none());
        assert!(welch_psd(&[], 44_100).is_none());
    }

    #[test]
    fn effective_bits_from_trailing_zeros() {
        let padded = IntSampleStats {
            min_trailing_zeros: 16,
            nonzero_samples: 200_000,
        };
        assert_eq!(effective_bits(&padded), Some(16));

        let dithered = IntSampleStats {
            min_trailing_zeros: 8,
            nonzero_samples: 200_000,
        };
        assert_eq!(effective_bits(&dithered), Some(24));

        let sparse = IntSampleStats {
            min_trailing_zeros: 16,
            nonzero_samples: 99_999,
        };
        assert_eq!(effective_bits(&sparse), None);
    }
}
