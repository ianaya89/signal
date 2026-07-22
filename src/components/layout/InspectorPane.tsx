const FIELDS = [
  "codec",
  "container",
  "bitrate",
  "bit depth",
  "sample rate",
  "channels",
  "replaygain",
  "peak",
  "dr",
  "output device",
  "output rate",
  "bit perfect",
] as const;

export function InspectorPane() {
  return (
    <dl className="py-1">
      {FIELDS.map((field) => (
        <div
          key={field}
          className="flex h-6 items-center justify-between px-2"
        >
          <dt className="text-[11px] text-muted">{field}</dt>
          <dd className="text-[11px] text-secondary">—</dd>
        </div>
      ))}
    </dl>
  );
}
