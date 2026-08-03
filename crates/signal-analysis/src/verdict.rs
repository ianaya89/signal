//! Threshold layer: spectral metrics + bit stats → a verdict with evidence.
//! Every tunable lives in the constants below.

use crate::dsp::SpectralMetrics;
use crate::{AnalysisResult, Verdict};

const HIRES_MIN_SR: u32 = 88_200;
/// Genuine hi-res content extends past this; 44.1k sources cannot.
const CD_CONTENT_CEILING_HZ: u32 = 23_000;
/// Lossy encoder lowpass range, 128 kbps (≈16 k) up to 320 kbps (≈20.5 k).
const TRANSCODE_BAND_HZ: (u32, u32) = (15_000, 20_800);
/// Analog/mic rolloff is gradual; lossy lowpass filters drop far more than
/// this within a kilohertz.
const CLIFF_MIN_DB: f64 = 25.0;
const PADDED_MIN_CLAIMED: u8 = 17;
const PADDED_EFFECTIVE_MAX: u8 = 16;

pub(crate) fn classify(
    sample_rate: u32,
    claimed_bit_depth: Option<u8>,
    metrics: &SpectralMetrics,
    effective_bits: Option<u8>,
) -> AnalysisResult {
    if metrics.too_quiet {
        return AnalysisResult {
            verdict: Verdict::Clean,
            cutoff_hz: None,
            effective_bit_depth: effective_bits,
            cliff_db: None,
            confidence: 0.0,
            detail: "too quiet to analyze reliably".into(),
        };
    }

    // Requires a real margin below the claim so borderline dither patterns
    // don't get called fraud.
    let padded = matches!(
        (claimed_bit_depth, effective_bits),
        (Some(claimed), Some(effective))
            if claimed >= PADDED_MIN_CLAIMED
                && effective <= PADDED_EFFECTIVE_MAX
                && effective + 4 < claimed
    );
    let padded_note = |claimed: Option<u8>, effective: Option<u8>| -> String {
        match (claimed, effective) {
            (Some(c), Some(e)) if padded => {
                format!("; also {c}-bit container with only {e} effective bits")
            }
            _ => String::new(),
        }
    };

    let mut result = AnalysisResult {
        verdict: Verdict::Clean,
        cutoff_hz: metrics.cutoff_hz,
        effective_bit_depth: effective_bits,
        cliff_db: metrics.cliff_db,
        confidence: 1.0,
        detail: String::new(),
    };

    if let (Some(cutoff), Some(cliff)) = (metrics.cutoff_hz, metrics.cliff_db) {
        if sample_rate >= HIRES_MIN_SR && cutoff < CD_CONTENT_CEILING_HZ && cliff >= CLIFF_MIN_DB {
            result.verdict = Verdict::Upsampled;
            result.confidence = (0.5
                + f64::from(CD_CONTENT_CEILING_HZ - cutoff) / 10_000.0
                + (cliff - CLIFF_MIN_DB) / 50.0)
                .min(1.0);
            result.detail = format!(
                "content stops at {} with a {cliff:.0} dB cliff — a real {} recording extends past 23 kHz; likely upsampled from a CD-rate source{}",
                khz(cutoff),
                khz(sample_rate),
                padded_note(claimed_bit_depth, effective_bits)
            );
            return result;
        }
        if sample_rate >= 44_100
            && (TRANSCODE_BAND_HZ.0..=TRANSCODE_BAND_HZ.1).contains(&cutoff)
            && cliff >= CLIFF_MIN_DB
        {
            let ancestor = if cutoff < 16_500 {
                "~128 kbps"
            } else if cutoff < 19_500 {
                "~192 kbps"
            } else {
                "~256–320 kbps"
            };
            result.verdict = Verdict::Transcode;
            result.confidence = (0.5 + (cliff - CLIFF_MIN_DB) / 50.0).min(1.0);
            result.detail = format!(
                "spectral cliff at {} ({cliff:.0} dB) matches a {ancestor} MP3/AAC ancestor{}",
                khz(cutoff),
                padded_note(claimed_bit_depth, effective_bits)
            );
            return result;
        }
    }

    if padded {
        if let (Some(claimed), Some(effective)) = (claimed_bit_depth, effective_bits) {
            result.verdict = Verdict::PaddedBits;
            result.confidence = 0.95;
            result.detail = format!(
                "{claimed}-bit container but only {effective} effective bits — the lower bits are zero in every sampled frame"
            );
            return result;
        }
    }

    result.detail = match (metrics.cutoff_hz, effective_bits) {
        (Some(cutoff), Some(bits)) => format!("content to {}, {bits} effective bits", khz(cutoff)),
        (Some(cutoff), None) => format!("content to {}", khz(cutoff)),
        _ => "no measurable content ceiling".into(),
    };
    result
}

fn khz(hz: u32) -> String {
    format!("{:.1} kHz", f64::from(hz) / 1_000.0)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    fn metrics(cutoff_hz: Option<u32>, cliff_db: Option<f64>) -> SpectralMetrics {
        SpectralMetrics {
            too_quiet: false,
            cutoff_hz,
            cliff_db,
        }
    }

    #[test]
    fn upsampled_when_hires_content_stops_at_cd_band() {
        let r = classify(96_000, Some(24), &metrics(Some(20_100), Some(60.0)), Some(24));
        assert_eq!(r.verdict, Verdict::Upsampled);
        assert!(r.confidence > 0.7);
        assert!(r.detail.contains("20.1 kHz"), "{}", r.detail);
    }

    #[test]
    fn hires_full_band_is_clean() {
        let r = classify(96_000, Some(24), &metrics(Some(45_000), None), Some(24));
        assert_eq!(r.verdict, Verdict::Clean);
    }

    #[test]
    fn transcode_named_by_ancestor_bitrate() {
        let r = classify(44_100, Some(16), &metrics(Some(16_000), Some(52.0)), Some(16));
        assert_eq!(r.verdict, Verdict::Transcode);
        assert!(r.detail.contains("~128 kbps"), "{}", r.detail);

        let r = classify(44_100, Some(16), &metrics(Some(20_300), Some(40.0)), Some(16));
        assert_eq!(r.verdict, Verdict::Transcode);
        assert!(r.detail.contains("256–320"), "{}", r.detail);
    }

    #[test]
    fn gentle_cliff_stays_clean() {
        let r = classify(44_100, Some(16), &metrics(Some(17_000), Some(12.0)), Some(16));
        assert_eq!(r.verdict, Verdict::Clean);
    }

    #[test]
    fn low_sample_rates_never_get_transcode_verdicts() {
        // Nyquist sits inside the transcode band — nothing to distinguish.
        let r = classify(32_000, Some(16), &metrics(Some(15_500), Some(50.0)), Some(16));
        assert_eq!(r.verdict, Verdict::Clean);
    }

    #[test]
    fn padded_bits_is_reported_alone() {
        let r = classify(44_100, Some(24), &metrics(Some(21_500), None), Some(16));
        assert_eq!(r.verdict, Verdict::PaddedBits);
        assert!(r.detail.contains("24-bit container"), "{}", r.detail);
    }

    #[test]
    fn padded_bits_rides_along_on_upsampled() {
        let r = classify(96_000, Some(24), &metrics(Some(20_000), Some(55.0)), Some(16));
        assert_eq!(r.verdict, Verdict::Upsampled);
        assert!(r.detail.contains("effective bits"), "{}", r.detail);
    }

    #[test]
    fn sixteen_bit_claims_are_never_padded() {
        let r = classify(44_100, Some(16), &metrics(Some(21_000), None), Some(16));
        assert_eq!(r.verdict, Verdict::Clean);
    }

    #[test]
    fn too_quiet_short_circuits() {
        let m = SpectralMetrics {
            too_quiet: true,
            cutoff_hz: None,
            cliff_db: None,
        };
        let r = classify(96_000, Some(24), &m, Some(16));
        assert_eq!(r.verdict, Verdict::Clean);
        assert!(r.detail.contains("too quiet"), "{}", r.detail);
    }
}
