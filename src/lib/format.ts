export function fmtDuration(ms: number): string {
  const totalSec = Math.round(ms / 1000);
  const min = Math.floor(totalSec / 60);
  const sec = totalSec % 60;
  return `${min}:${sec.toString().padStart(2, "0")}`;
}

export function fmtSampleRate(hz: number): string {
  const khz = hz / 1000;
  return `${Number.isInteger(khz) ? khz : khz.toFixed(1)}kHz`;
}

/** "16/44.1" style bit-depth/sample-rate badge text. */
export function fmtQuality(bitDepth: number | null, sampleRateHz: number): string {
  const khz = sampleRateHz / 1000;
  const rate = Number.isInteger(khz) ? String(khz) : khz.toFixed(1);
  return bitDepth ? `${bitDepth}/${rate}` : rate;
}

const LOSSY_CODECS = new Set(["MP3", "AAC", "Opus", "Vorbis"]);

export function isLossy(codec: string): boolean {
  return LOSSY_CODECS.has(codec);
}

export function isHires(bitDepth: number | null, sampleRateHz: number): boolean {
  return (bitDepth ?? 0) >= 24 || sampleRateHz > 48_000;
}
