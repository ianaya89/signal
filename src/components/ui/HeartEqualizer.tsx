import { useMemo } from "react";

import { buildHeartBars } from "@/lib/heart";

/** The bar-heart mark as a living equalizer: bars pulse while playing,
 *  freeze when paused. Pure CSS, sizes via the `size` prop (px). */
export function HeartEqualizer({
  size,
  playing,
}: {
  size: number;
  playing: boolean;
}) {
  const bars = useMemo(() => buildHeartBars(), []);
  return (
    <div
      aria-hidden
      className="relative"
      style={{ width: size, height: size }}
    >
      {bars.map((bar, i) => (
        <span
          key={i}
          className="eq-bar absolute rounded-full"
          style={{
            left: bar.x * size,
            top: bar.top * size,
            width: bar.width * size,
            height: bar.height * size,
            background: bar.color,
            animationDuration: `${0.6 + (i % 4) * 0.17}s`,
            animationDelay: `${(i % 5) * 90}ms`,
            animationPlayState: playing ? "running" : "paused",
          }}
        />
      ))}
    </div>
  );
}

/** Small linear equalizer for inline "now playing" spots. */
export function EqBars({
  playing,
  className,
}: {
  playing: boolean;
  className?: string;
}) {
  return (
    <span
      aria-hidden
      className={className}
      style={{ display: "inline-flex", alignItems: "flex-end", gap: 2, height: 12 }}
    >
      {[7, 11, 5, 9].map((h, i) => (
        <span
          key={i}
          className="eq-bar inline-block w-[3px] bg-accent"
          style={{
            height: h,
            animationDuration: `${0.55 + i * 0.14}s`,
            animationDelay: `${i * 120}ms`,
            animationPlayState: playing ? "running" : "paused",
          }}
        />
      ))}
    </span>
  );
}
