import { useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { useState } from "react";

import { useMainTitle } from "@/hooks/useMainTitle";
import { api } from "@/ipc/invoke";
import type { AlbumPlayCount, DayCount } from "@/ipc/types";
import { artworkUrl } from "@/lib/artwork";

export function StatsView() {
  useMainTitle("stats");
  const { data, isLoading } = useQuery({
    queryKey: ["stats"],
    queryFn: api.statsOverview,
    refetchOnWindowFocus: true,
  });

  if (isLoading || !data) {
    return <p className="p-3 text-muted">loading…</p>;
  }

  if (data.totalPlays === 0) {
    return (
      <p className="p-3 text-[12px] text-muted">
        no listening history yet — play something
      </p>
    );
  }

  return (
    <div className="flex flex-col gap-4 overflow-auto p-3">
      <section className="flex gap-6">
        <Stat label="plays" value={String(data.totalPlays)} />
        <Stat label="time listened" value={fmtHours(data.totalMsPlayed)} />
        <Stat label="distinct tracks" value={String(data.distinctTracks)} />
      </section>

      <section>
        <h2 className="mb-1 text-[11px] text-muted">last 26 weeks</h2>
        <Heatmap days={data.heatmap} />
      </section>

      {data.topAlbums.length > 0 && (
        <section>
          <h2 className="mb-1 text-[11px] text-muted">top albums</h2>
          <div className="flex flex-wrap gap-2">
            {data.topAlbums.map((album) => (
              <TopAlbumCard key={album.albumId} album={album} />
            ))}
          </div>
        </section>
      )}

      <section>
        <h2 className="mb-1 text-[11px] text-muted">plays by hour</h2>
        <HourlyChart hourly={data.hourly} />
      </section>

      <div className="flex gap-8">
        <TopList title="top artists" items={data.topArtists} />
        <TopList title="top codecs" items={data.topCodecs} />
      </div>
    </div>
  );
}

function TopAlbumCard({ album }: { album: AlbumPlayCount }) {
  const [artError, setArtError] = useState(false);
  return (
    <Link
      to="/albums/$albumId"
      params={{ albumId: String(album.albumId) }}
      className="group w-24"
      title={`${album.name} — ${album.artistName} · ${album.plays} plays`}
    >
      <div className="aspect-square overflow-hidden border border-subtle bg-raised group-hover:border-focus">
        {!artError ? (
          <img
            src={artworkUrl(album.albumId)}
            alt=""
            loading="lazy"
            onError={() => setArtError(true)}
            className="h-full w-full object-cover"
          />
        ) : (
          <div className="flex h-full w-full items-center justify-center text-muted">
            ♪
          </div>
        )}
      </div>
      <div className="truncate text-[10px] text-secondary group-hover:text-accent">
        {album.name}
      </div>
      <div className="text-[10px] text-muted">{album.plays} plays</div>
    </Link>
  );
}

function HourlyChart({ hourly }: { hourly: number[] }) {
  const max = Math.max(1, ...hourly);
  return (
    <div className="flex h-16 items-end gap-px">
      {hourly.map((count, hour) => (
        <div
          key={hour}
          title={`${hour}:00 — ${count} plays`}
          className="flex flex-1 flex-col items-center gap-0.5"
        >
          <div
            className="w-full bg-accent-dim hover:bg-accent"
            style={{ height: `${(count / max) * 52}px` }}
          />
          {hour % 6 === 0 && (
            <span className="text-[8px] text-muted">{hour}</span>
          )}
        </div>
      ))}
    </div>
  );
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <div className="text-[18px] text-primary">{value}</div>
      <div className="text-[11px] text-muted">{label}</div>
    </div>
  );
}

function TopList({
  title,
  items,
}: {
  title: string;
  items: { name: string; count: number }[];
}) {
  if (items.length === 0) return null;
  const max = items[0]?.count ?? 1;
  return (
    <section className="min-w-56">
      <h2 className="mb-1 text-[11px] text-muted">{title}</h2>
      <ul className="flex flex-col gap-px">
        {items.map((item) => (
          <li key={item.name} className="flex h-5 items-center gap-2">
            <span className="w-32 truncate text-[11px] text-secondary">
              {item.name}
            </span>
            <div className="h-2 flex-1 bg-raised">
              <div
                className="h-full bg-accent-dim"
                style={{ width: `${(item.count / max) * 100}%` }}
              />
            </div>
            <span className="w-8 text-right text-[11px] text-muted">
              {item.count}
            </span>
          </li>
        ))}
      </ul>
    </section>
  );
}

/** GitHub-style contribution grid: 26 weeks × 7 days, accent alpha ramp. */
function Heatmap({ days }: { days: DayCount[] }) {
  const byDay = new Map(days.map((d) => [d.day, d.plays]));
  const max = Math.max(1, ...days.map((d) => d.plays));

  const today = new Date();
  const cells: { date: string; plays: number }[] = [];
  for (let i = 26 * 7 - 1; i >= 0; i--) {
    const d = new Date(today);
    d.setDate(d.getDate() - i);
    const key = d.toISOString().slice(0, 10);
    cells.push({ date: key, plays: byDay.get(key) ?? 0 });
  }

  const weeks: (typeof cells)[] = [];
  for (let w = 0; w < 26; w++) {
    weeks.push(cells.slice(w * 7, w * 7 + 7));
  }

  return (
    <div className="flex gap-px">
      {weeks.map((week) => (
        <div key={week[0]?.date} className="flex flex-col gap-px">
          {week.map((cell) => (
            <div
              key={cell.date}
              title={`${cell.date}: ${cell.plays} plays`}
              className="h-2.5 w-2.5"
              style={{
                background:
                  cell.plays === 0
                    ? "var(--bg-raised)"
                    : `color-mix(in srgb, var(--accent) ${
                        20 + Math.round((cell.plays / max) * 80)
                      }%, var(--bg-base))`,
              }}
            />
          ))}
        </div>
      ))}
    </div>
  );
}

function fmtHours(ms: number): string {
  const hours = ms / 3_600_000;
  if (hours < 1) return `${Math.round(ms / 60_000)}m`;
  return `${hours.toFixed(1)}h`;
}
