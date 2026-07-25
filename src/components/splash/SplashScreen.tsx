import { useEffect, useMemo, useState } from "react";

// Mirrors src-tauri/icons/make_logo.py: column extents of the implicit
// heart (x²+y²-1)³ - x²y³ ≤ 0, with deterministic per-bar jitter.
const N_BARS = 11;
const CANVAS = 220;
const SCALE = CANVAS * 0.3;
const CY = CANVAS * 0.54;
const BAR_W = CANVAS * 0.052;
const GAP = CANVAS * 0.026;

interface Bar {
  x: number;
  top: number;
  height: number;
  color: string;
  delay: number;
  wobbleDelay: number;
}

function heartExtents(x: number): [number, number] | null {
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

// deterministic jitter, no RNG state
const jitter = (i: number, salt: number) =>
  Math.sin(i * 7.3 + salt * 13.7) * 0.07;

function lerpColor(t: number): string {
  const a = [0x82, 0x86, 0xf5];
  const b = [0xbb, 0x9a, 0xf7];
  const c = a.map((v, i) => Math.round(v + ((b[i] ?? v) - v) * t));
  return `rgb(${c[0]},${c[1]},${c[2]})`;
}

function buildBars(): Bar[] {
  const total = N_BARS * BAR_W + (N_BARS - 1) * GAP;
  const x0 = (CANVAS - total) / 2;
  const bars: Bar[] = [];
  for (let i = 0; i < N_BARS; i++) {
    const cx = x0 + i * (BAR_W + GAP) + BAR_W / 2;
    const hx = (cx - CANVAS / 2) / SCALE;
    const ext = heartExtents(hx);
    if (!ext) continue;
    const [lo, hi] = ext;
    const top = CY - (hi + jitter(i, 1)) * SCALE;
    const bottom = CY - (lo + jitter(i, 2)) * SCALE;
    // stagger from the center outward, like a heartbeat spreading
    const fromCenter = Math.abs(i - (N_BARS - 1) / 2);
    bars.push({
      x: cx - BAR_W / 2,
      top,
      height: Math.max(bottom - top, BAR_W),
      color: lerpColor(i / (N_BARS - 1)),
      delay: fromCenter * 70,
      wobbleDelay: i * 90,
    });
  }
  return bars;
}

const TOTAL_MS = 2300;

export function SplashScreen({ onDone }: { onDone: () => void }) {
  const bars = useMemo(buildBars, []);
  const [leaving, setLeaving] = useState(false);
  const reduced = useMemo(
    () => window.matchMedia("(prefers-reduced-motion: reduce)").matches,
    [],
  );

  useEffect(() => {
    const total = reduced ? 700 : TOTAL_MS;
    const fade = setTimeout(() => setLeaving(true), total - 400);
    const done = setTimeout(onDone, total);
    return () => {
      clearTimeout(fade);
      clearTimeout(done);
    };
  }, [onDone, reduced]);

  return (
    <div
      className="absolute inset-0 z-[100] flex flex-col items-center justify-center bg-base transition-opacity duration-300"
      style={{ opacity: leaving ? 0 : 1 }}
    >
      <style>{`
        @keyframes splash-grow {
          0% { transform: scaleY(0); }
          70% { transform: scaleY(1.12); }
          100% { transform: scaleY(1); }
        }
        @keyframes splash-wobble {
          0%, 100% { transform: scaleY(1); }
          35% { transform: scaleY(0.9); }
          65% { transform: scaleY(1.07); }
        }
        @keyframes splash-fade-in {
          from { opacity: 0; }
          to { opacity: 1; }
        }
        @media (prefers-reduced-motion: reduce) {
          .splash-bar { animation: none !important; }
        }
      `}</style>
      <div
        className="relative"
        style={{ width: CANVAS, height: CANVAS }}
        aria-hidden
      >
        {bars.map((bar, i) => (
          <div
            key={i}
            className="splash-bar absolute rounded-full"
            style={{
              left: bar.x,
              top: bar.top,
              width: BAR_W,
              height: bar.height,
              background: bar.color,
              transformOrigin: "center",
              animation: [
                `splash-grow 480ms cubic-bezier(0.34, 1.3, 0.64, 1) ${bar.delay}ms both`,
                `splash-wobble 900ms ease-in-out ${700 + bar.wobbleDelay}ms 1`,
              ].join(", "),
            }}
          />
        ))}
      </div>
      <p
        className="mt-4 text-[13px] text-secondary"
        style={{ animation: "splash-fade-in 500ms ease-out 600ms both" }}
      >
        <span className="text-accent">❯</span> signal
      </p>
    </div>
  );
}
