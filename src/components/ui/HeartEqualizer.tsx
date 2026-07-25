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
          // paused: no animation at all → bars sit at scale 1, a clean heart
          className={playing ? "eq-bar-soft absolute rounded-full" : "absolute rounded-full"}
          style={{
            left: bar.x * size,
            top: bar.top * size,
            width: bar.width * size,
            height: bar.height * size,
            background: bar.color,
            ...(playing
              ? {
                  animationDuration: `${0.7 + (i % 4) * 0.16}s`,
                  animationDelay: `${(i % 5) * 110}ms`,
                }
              : null),
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
