import { buildHeartBars } from "@/lib/heart";
import { cn } from "@/lib/utils";

function hashOf(seed: string): number {
  let h = 2166136261;
  for (const ch of seed) {
    h ^= ch.charCodeAt(0);
    h = Math.imul(h, 16777619);
  }
  return h >>> 0;
}

const BARS = buildHeartBars();

/** Missing-artwork tile: the bar-heart mark blown up and cropped
 *  off-grid, tinted per album name (deterministic), with the name as a
 *  terminal prompt. Small variant centers the heart instead. */
export function CoverPlaceholder({
  name,
  showName = true,
  className,
}: {
  name: string;
  showName?: boolean;
  className?: string;
}) {
  const hash = hashOf(name);
  const hue = hash % 360;
  const dx = 12 + ((hash >> 3) % 28);
  const dy = 6 + ((hash >> 8) % 24);
  const tilt = -10 + ((hash >> 5) % 21);
  const flip = ((hash >> 11) & 1) === 1;

  const heart = (
    <div
      aria-hidden
      className="absolute"
      style={
        showName
          ? {
              width: "115%",
              height: "115%",
              top: `-${dy}%`,
              [flip ? "left" : "right"]: `-${dx}%`,
              transform: `rotate(${tilt}deg)`,
              opacity: 0.9,
            }
          : { width: "78%", height: "78%", left: "11%", top: "11%" }
      }
    >
      {BARS.map((bar, i) => (
        <span
          key={i}
          className="absolute rounded-full"
          style={{
            left: `${bar.x * 100}%`,
            top: `${bar.top * 100}%`,
            width: `${bar.width * 100}%`,
            height: `${bar.height * 100}%`,
            background: `hsl(${(hue + i * 6) % 360} 60% ${46 + i * 2}%)`,
          }}
        />
      ))}
    </div>
  );

  return (
    <div
      className={cn("relative h-full w-full overflow-hidden", className)}
      style={{
        background: `linear-gradient(160deg, hsl(${hue} 45% 19%), hsl(${(hue + 40) % 360} 55% 10%))`,
      }}
    >
      {heart}
      {showName && (
        <>
          <div
            aria-hidden
            className="absolute inset-x-0 bottom-0 h-1/3"
            style={{
              background: `linear-gradient(transparent, hsl(${hue} 50% 6% / 0.9))`,
            }}
          />
          <span className="absolute inset-x-1.5 bottom-1 truncate text-[10px] leading-tight">
            <span style={{ color: `hsl(${hue} 75% 72%)` }}>❯ </span>
            <span style={{ color: `hsl(${hue} 20% 90%)` }}>{name}</span>
          </span>
        </>
      )}
    </div>
  );
}
