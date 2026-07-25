import { useQuery } from "@tanstack/react-query";

import { api } from "@/ipc/invoke";
import { fmtQuality, fmtSampleRate } from "@/lib/format";
import { cn } from "@/lib/utils";
import { usePlayerStore } from "@/stores/playerStore";

/** The audio path, stage by stage: file → decode → dsp → output → device.
 *  Rates and formats shown at every hop; the lossy step, if any, glows. */
export function ChainView() {
  const {
    trackId,
    status,
    replaygain,
    exclusive,
    volume,
    bitPerfect,
    sourceRateHz,
    outputRateHz,
    decodedFormat,
    outputFormat,
    ao,
    deviceId,
  } = usePlayerStore();

  const { data: track } = useQuery({
    queryKey: ["track", trackId],
    queryFn: () => api.getTrack(trackId ?? -1),
    enabled: trackId !== null,
    staleTime: Infinity,
  });
  const { data: devices } = useQuery({
    queryKey: ["devices"],
    queryFn: api.deviceList,
    staleTime: 30_000,
  });

  if (trackId === null || status === "stopped" || !track) {
    return null;
  }

  const t = track.track.technical;
  const volumePct = Math.round(volume * 100);
  const dspActive = volumePct !== 100 || replaygain !== "off";
  const resampled =
    sourceRateHz !== null && outputRateHz !== null && sourceRateHz !== outputRateHz;
  const deviceName =
    devices?.find((d) => d.id === deviceId)?.name ?? "system default";

  return (
    <div className="border-t border-subtle px-2 py-2">
      <h3 className="mb-1.5 text-[10px] uppercase tracking-wider text-muted">
        audio chain
      </h3>
      <div className="flex flex-col">
        <Stage
          label="file"
          value={`${t.codec} · ${fmtQuality(t.bitDepth, t.sampleRateHz)} · ${t.channels}ch`}
        />
        <Hop />
        <Stage
          label="decode"
          value={`${decodedFormat ?? "…"} @ ${
            sourceRateHz ? fmtSampleRate(sourceRateHz) : "…"
          }`}
        />
        <Hop />
        <Stage
          label="dsp"
          value={
            dspActive
              ? [
                  volumePct !== 100 ? `volume ${volumePct}%` : null,
                  replaygain !== "off" ? `rg ${replaygain}` : null,
                ]
                  .filter(Boolean)
                  .join(" · ")
              : "bypass"
          }
          tone={dspActive ? "warn" : "ok"}
        />
        <Hop />
        <Stage
          label="output"
          value={`${ao ?? "…"} · ${outputFormat ?? "…"} @ ${
            outputRateHz ? fmtSampleRate(outputRateHz) : "…"
          }${exclusive ? " · exclusive" : ""}`}
          tone={resampled ? "warn" : undefined}
        />
        <Hop />
        <Stage
          label="device"
          value={deviceName}
          badge={
            bitPerfect ? (
              <span className="text-[10px] text-bitperfect">● bit-perfect</span>
            ) : resampled ? (
              <span className="text-[10px] text-warn">
                resampled {fmtSampleRate(sourceRateHz ?? 0)}→
                {fmtSampleRate(outputRateHz ?? 0)}
              </span>
            ) : (
              <span className="text-[10px] text-warn">○ dsp active</span>
            )
          }
        />
      </div>
    </div>
  );
}

function Stage({
  label,
  value,
  tone,
  badge,
}: {
  label: string;
  value: string;
  tone?: "warn" | "ok";
  badge?: React.ReactNode;
}) {
  return (
    <div className="flex items-baseline gap-2">
      <span className="w-14 shrink-0 text-[10px] text-muted">{label}</span>
      <span
        className={cn(
          "min-w-0 flex-1 truncate text-[11px]",
          tone === "warn"
            ? "text-warn"
            : tone === "ok"
              ? "text-ok"
              : "text-secondary",
        )}
      >
        {value}
      </span>
      {badge}
    </div>
  );
}

function Hop() {
  return <span className="ml-14 pl-2 text-[9px] leading-3 text-muted">│</span>;
}
