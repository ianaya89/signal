import { useQuery } from "@tanstack/react-query";

import { ChainView } from "@/components/layout/ChainView";
import { Loading } from "@/components/ui/States";
import { api } from "@/ipc/invoke";
import type { ReplayGainMode } from "@/ipc/types";
import { fmtSampleRate } from "@/lib/format";
import { cn } from "@/lib/utils";
import { usePlayerStore } from "@/stores/playerStore";

export function InspectorPane() {
  const trackId = usePlayerStore((s) => s.trackId);

  return (
    <div className="flex flex-col">
      {trackId === null ? (
        <p className="p-3 text-[11px] text-muted">
          nothing playing — technical data appears here
        </p>
      ) : (
        <TrackInspector trackId={trackId} />
      )}
      <ChainView />
      <OutputSection />
    </div>
  );
}

const RG_MODES: ReplayGainMode[] = ["off", "track", "album"];

function OutputSection() {
  const { replaygain, exclusive, bitPerfect, sourceRateHz, outputRateHz, deviceId, status } =
    usePlayerStore();
  const { data: devices } = useQuery({
    queryKey: ["devices"],
    queryFn: api.deviceList,
    staleTime: 30_000,
  });

  return (
    <div className="border-t border-subtle px-2 py-2">
      <h3 className="mb-1 text-[10px] uppercase tracking-wider text-muted">
        output
      </h3>

      <label className="mb-1 flex items-center justify-between gap-2">
        <span className="text-[11px] text-muted">device</span>
        <select
          value={deviceId ?? "auto"}
          onChange={(e) => void api.deviceSelect(e.target.value)}
          className="w-36 truncate rounded-[var(--radius-sm)] border border-subtle bg-base/60 px-1 py-0.5 text-[11px] text-secondary outline-none focus:border-focus"
        >
          {!devices?.some((d) => d.id === (deviceId ?? "auto")) && (
            <option value={deviceId ?? "auto"}>auto</option>
          )}
          {devices?.map((d) => (
            <option key={d.id} value={d.id}>
              {d.name}
            </option>
          ))}
        </select>
      </label>

      <div className="mb-1 flex items-center justify-between">
        <span className="text-[11px] text-muted">replaygain</span>
        <div className="flex gap-px overflow-hidden rounded-[var(--radius-sm)] border border-subtle">
          {RG_MODES.map((mode) => (
            <button
              key={mode}
              type="button"
              onClick={() => void api.setReplaygain(mode)}
              className={cn(
                "px-1.5 py-0.5 text-[10px]",
                replaygain === mode
                  ? "bg-raised text-accent"
                  : "text-muted hover:text-secondary",
              )}
            >
              {mode}
            </button>
          ))}
        </div>
      </div>

      <div className="mb-1 flex items-center justify-between">
        <span className="text-[11px] text-muted">exclusive</span>
        <button
          type="button"
          onClick={() => void api.setExclusive(!exclusive)}
          className={cn(
            "rounded-[var(--radius-sm)] border border-subtle px-1.5 py-0.5 text-[10px]",
            exclusive ? "bg-raised text-accent" : "text-muted hover:text-secondary",
          )}
        >
          {exclusive ? "on" : "off"}
        </button>
      </div>

      {status !== "stopped" && (
        <div className="flex items-center justify-between">
          <span className="text-[11px] text-muted">bit perfect</span>
          <span
            className={cn(
              "text-[11px]",
              bitPerfect ? "text-bitperfect" : "text-warn",
            )}
            title={
              sourceRateHz && outputRateHz
                ? `${fmtSampleRate(sourceRateHz)} → ${fmtSampleRate(outputRateHz)}`
                : undefined
            }
          >
            {bitPerfect
              ? "● yes"
              : sourceRateHz && outputRateHz && sourceRateHz !== outputRateHz
                ? `resampled ${fmtSampleRate(sourceRateHz)}→${fmtSampleRate(outputRateHz)}`
                : "○ no (dsp active)"}
          </span>
        </div>
      )}
    </div>
  );
}

function TrackInspector({ trackId }: { trackId: number }) {
  const { data } = useQuery({
    queryKey: ["track", trackId],
    queryFn: () => api.getTrack(trackId),
    staleTime: Infinity,
  });

  if (!data) {
    return <Loading />;
  }

  const t = data.track.technical;
  const fields: [string, string | null, string?][] = [
    ["codec", t.codec],
    ["container", t.container],
    ["bitrate", t.bitrateKbps ? `${t.bitrateKbps} kbps` : null],
    ["bit depth", t.bitDepth ? `${t.bitDepth} bit` : null],
    ["sample rate", fmtSampleRate(t.sampleRateHz)],
    ["channels", String(t.channels)],
    [
      "rg track",
      t.replaygainTrackGain !== null ? `${t.replaygainTrackGain.toFixed(2)} dB` : null,
    ],
    [
      "rg album",
      t.replaygainAlbumGain !== null ? `${t.replaygainAlbumGain.toFixed(2)} dB` : null,
    ],
    ["peak", t.peak !== null ? t.peak.toFixed(6) : null],
    ["dr", t.drScore !== null ? String(t.drScore) : null],
    ["encoder", t.encoder],
    ["size", fmtBytes(t.fileSizeBytes)],
    ["md5", t.md5 ? t.md5.slice(0, 12) : null],
  ];

  return (
    <div className="flex h-full flex-col">
      <dl className="py-1">
        {fields.map(([label, value, valueClass]) => (
          <div key={label} className="flex h-6 items-center justify-between gap-2 px-2">
            <dt className="shrink-0 text-[11px] text-muted">{label}</dt>
            <dd
              className={cn(
                "truncate text-[11px]",
                value ? (valueClass ?? "text-secondary") : "text-muted",
              )}
            >
              {value ?? "—"}
            </dd>
          </div>
        ))}
      </dl>
      <div className="mt-auto border-t border-subtle p-2">
        <button
          type="button"
          onClick={() => void api.revealFile(t.filePath)}
          title="reveal in finder"
          className="w-full truncate text-left text-[10px] text-muted hover:text-accent"
        >
          {t.filePath}
        </button>
      </div>
    </div>
  );
}

function fmtBytes(bytes: number): string {
  if (bytes >= 1_000_000_000) return `${(bytes / 1_000_000_000).toFixed(2)} GB`;
  if (bytes >= 1_000_000) return `${(bytes / 1_000_000).toFixed(1)} MB`;
  return `${Math.round(bytes / 1000)} kB`;
}
