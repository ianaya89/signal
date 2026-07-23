import { useQuery } from "@tanstack/react-query";

import { api } from "@/ipc/invoke";
import { fmtSampleRate } from "@/lib/format";
import { cn } from "@/lib/utils";
import { usePlayerStore } from "@/stores/playerStore";

export function InspectorPane() {
  const trackId = usePlayerStore((s) => s.trackId);

  if (trackId === null) {
    return (
      <p className="p-3 text-[11px] text-muted">
        nothing playing — technical data appears here
      </p>
    );
  }
  return <TrackInspector trackId={trackId} />;
}

function TrackInspector({ trackId }: { trackId: number }) {
  const { data } = useQuery({
    queryKey: ["track", trackId],
    queryFn: () => api.getTrack(trackId),
    staleTime: Infinity,
  });

  if (!data) {
    return <p className="p-3 text-[11px] text-muted">loading…</p>;
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
        <p className="truncate text-[10px] text-muted" title={t.filePath}>
          {t.filePath}
        </p>
      </div>
    </div>
  );
}

function fmtBytes(bytes: number): string {
  if (bytes >= 1_000_000_000) return `${(bytes / 1_000_000_000).toFixed(2)} GB`;
  if (bytes >= 1_000_000) return `${(bytes / 1_000_000).toFixed(1)} MB`;
  return `${Math.round(bytes / 1000)} kB`;
}
