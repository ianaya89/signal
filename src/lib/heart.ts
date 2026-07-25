// Shared geometry for the bar-heart mark (icon, splash, equalizers).

/** Column extents of the implicit heart (x²+y²-1)³ - x²y³ ≤ 0. */
export function heartExtents(x: number): [number, number] | null {
  let lo: number | null = null;
  let hi: number | null = null;
  for (let i = 0; i <= 700; i++) {
    const y = -1.3 + (2.8 * i) / 700;
    const v = (x * x + y * y - 1) ** 3 - x * x * y ** 3;
    if (v <= 0) {
      if (lo === null) lo = y;
      hi = y;
    }
  }
  return lo === null || hi === null ? null : [lo, hi];
}

/** periwinkle → violet ramp, t in [0,1]. */
export function heartColor(t: number): string {
  const a = [0x82, 0x86, 0xf5];
  const b = [0xbb, 0x9a, 0xf7];
  const c = a.map((v, i) => Math.round(v + ((b[i] ?? v) - v) * t));
  return `rgb(${c[0]},${c[1]},${c[2]})`;
}

export interface HeartBar {
  /** all in [0,1] fractions of the canvas */
  x: number;
  top: number;
  height: number;
  width: number;
  color: string;
}

/** Bars tracing the heart silhouette, coordinates as canvas fractions. */
export function buildHeartBars(nBars = 11): HeartBar[] {
  const barW = 0.052;
  const gap = 0.026;
  const scale = 0.3;
  const cy = 0.54;
  const total = nBars * barW + (nBars - 1) * gap;
  const x0 = (1 - total) / 2;

  const bars: HeartBar[] = [];
  for (let i = 0; i < nBars; i++) {
    const cx = x0 + i * (barW + gap) + barW / 2;
    const ext = heartExtents((cx - 0.5) / scale);
    if (!ext) continue;
    const [lo, hi] = ext;
    const top = cy - hi * scale;
    const bottom = cy - lo * scale;
    bars.push({
      x: cx - barW / 2,
      top,
      height: Math.max(bottom - top, barW),
      width: barW,
      color: heartColor(i / (nBars - 1)),
    });
  }
  return bars;
}
