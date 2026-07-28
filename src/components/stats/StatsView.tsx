import { useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { useState, type ReactNode } from "react";

import { CoverPlaceholder } from "@/components/ui/CoverPlaceholder";
import { useMainTitle } from "@/hooks/useMainTitle";
import { api } from "@/ipc/invoke";
import type {
  AlbumPlayCount,
  DayCount,
  LibrarySummary,
  NameCount,
  TrackPlayCount,
} from "@/ipc/types";
import { artworkUrl } from "@/lib/artwork";
import { isLossy } from "@/lib/format";
import { cn } from "@/lib/utils";

const WEEKDAYS = ["sun", "mon", "tue", "wed", "thu", "fri", "sat"];

export function StatsView() {
  useMainTitle("stats");
  const { data, isLoading, error } = useQuery({
    queryKey: ["stats"],
    queryFn: api.statsOverview,
    refetchOnWindowFocus: true,
  });

  if (isLoading || !data) {
    return (
      <p className="p-3 text-muted">
        {error ? `stats unavailable — ${String(error)}` : "loading…"}
      </p>
    );
  }

  const { library } = data;
  const silent = data.totalPlays === 0;
  const peakHour = data.hourly.indexOf(Math.max(...data.hourly));
  const peakDay = data.weekday.indexOf(Math.max(...data.weekday));

  return (
    <div className="flex flex-col gap-3 overflow-auto p-3">
      <Masthead
        plays={data.totalPlays}
        msPlayed={data.totalMsPlayed}
        tracks={data.distinctTracks}
        streak={data.streakCurrent}
        bestStreak={data.streakBest}
      />

      {silent ? (
        <p className="border border-subtle bg-surface p-3 text-[12px] text-muted">
          no listening history yet — play something and this panel fills in
        </p>
      ) : (
        <>
          <Panel
            title="activity"
            meta="last 26 weeks"
            index={1}
            right={<HeatLegend />}
          >
            <Heatmap days={data.heatmap} />
          </Panel>

          <div className="grid grid-cols-1 gap-3 xl:grid-cols-[3fr_2fr]">
            <Panel
              title="clock"
              meta={`peak ${String(peakHour).padStart(2, "0")}:00`}
              index={2}
            >
              <HourlyChart hourly={data.hourly} peak={peakHour} />
            </Panel>
            <Panel
              title="week"
              meta={`busiest ${WEEKDAYS[peakDay] ?? "—"}`}
              index={3}
            >
              <WeekdayChart weekday={data.weekday} peak={peakDay} />
            </Panel>
          </div>

          <div className="grid grid-cols-1 gap-3 xl:grid-cols-2">
            <Panel title="top tracks" index={4}>
              <TopTracks tracks={data.topTracks} />
            </Panel>
            <Panel title="top artists" index={5}>
              <Meters items={data.topArtists} />
            </Panel>
          </div>

          {data.topAlbums.length > 0 && (
            <Panel title="most played albums" index={6}>
              <TopAlbums albums={data.topAlbums} />
            </Panel>
          )}
        </>
      )}

      <div className="grid grid-cols-1 gap-3 xl:grid-cols-2">
        <Panel title="formats" meta={`${library.losslessPct}% lossless`} index={7}>
          <FormatPanel codecs={data.topCodecs} losslessPct={library.losslessPct} />
        </Panel>
        <Panel title="library" index={8}>
          <LibraryPanel library={library} />
        </Panel>
      </div>
    </div>
  );
}

/** Hero readout: the four numbers worth glancing at, console-style. */
function Masthead({
  plays,
  msPlayed,
  tracks,
  streak,
  bestStreak,
}: {
  plays: number;
  msPlayed: number;
  tracks: number;
  streak: number;
  bestStreak: number;
}) {
  const hours = msPlayed / 3_600_000;
  return (
    <section
      className="panel-settle grid grid-cols-2 gap-px border border-subtle bg-subtle sm:grid-cols-4"
      style={{ "--i": 0 } as React.CSSProperties}
    >
      <Readout value={fmtCount(plays)} unit="plays" />
      <Readout
        value={hours < 1 ? String(Math.round(msPlayed / 60_000)) : hours.toFixed(1)}
        unit={hours < 1 ? "minutes listened" : "hours listened"}
      />
      <Readout value={fmtCount(tracks)} unit="distinct tracks" />
      <Readout
        value={String(streak)}
        unit="day streak"
        note={bestStreak > 0 ? `best ${bestStreak}` : undefined}
        live={streak > 0}
      />
    </section>
  );
}

function Readout({
  value,
  unit,
  note,
  live = false,
}: {
  value: string;
  unit: string;
  note?: string;
  live?: boolean;
}) {
  return (
    <div className="flex flex-col gap-0.5 bg-surface px-3 py-2">
      <span
        className={cn(
          "text-[24px] leading-none tabular-nums",
          live ? "text-accent" : "text-primary",
        )}
      >
        {value}
      </span>
      <span className="flex items-baseline gap-1.5">
        <span className="text-[10px] uppercase tracking-[0.14em] text-muted">
          {unit}
        </span>
        {note && <span className="text-[10px] text-accent-dim">{note}</span>}
      </span>
    </div>
  );
}

/** Bordered box with a bracketed header and a rule running to the margin. */
function Panel({
  title,
  meta,
  right,
  index,
  children,
}: {
  title: string;
  meta?: string;
  right?: ReactNode;
  index: number;
  children: ReactNode;
}) {
  return (
    <section
      className="panel-settle border border-subtle bg-surface"
      style={{ "--i": index } as React.CSSProperties}
    >
      <header className="flex h-7 items-center gap-2 border-b border-subtle px-2">
        <h2 className="shrink-0 text-[10px] uppercase tracking-[0.14em] text-accent">
          [ {title} ]
        </h2>
        {meta && <span className="shrink-0 text-[10px] text-muted">{meta}</span>}
        <span className="h-px min-w-4 flex-1 bg-subtle" />
        {right}
      </header>
      <div className="p-3">{children}</div>
    </section>
  );
}

function HeatLegend() {
  return (
    <span className="flex shrink-0 items-center gap-1 text-[9px] text-muted">
      less
      {[0, 0.35, 0.6, 0.85, 1].map((t) => (
        <span
          key={t}
          className="h-2 w-2"
          style={{ background: heatColor(t) }}
          aria-hidden
        />
      ))}
      more
    </span>
  );
}

/** 26 weeks × 7 days, month ticks above, accent ramp. */
function Heatmap({ days }: { days: DayCount[] }) {
  const byDay = new Map(days.map((d) => [d.day, d.plays]));
  const max = Math.max(1, ...days.map((d) => d.plays));

  const today = new Date();
  const cells: { date: string; plays: number; month: number }[] = [];
  for (let i = 26 * 7 - 1; i >= 0; i--) {
    const d = new Date(today);
    d.setDate(d.getDate() - i);
    const key = toIsoDay(d);
    cells.push({ date: key, plays: byDay.get(key) ?? 0, month: d.getMonth() });
  }

  const weeks: (typeof cells)[] = [];
  for (let w = 0; w < 26; w++) {
    weeks.push(cells.slice(w * 7, w * 7 + 7));
  }

  return (
    <div className="flex gap-2 overflow-x-auto">
      <div className="mt-[13px] flex shrink-0 flex-col gap-px text-[8px] text-muted">
        {WEEKDAYS.map((label, i) => (
          <span key={label} className="h-2.5 leading-[10px]">
            {i % 2 === 1 ? label : ""}
          </span>
        ))}
      </div>
      <div className="flex gap-px">
        {weeks.map((week, w) => {
          const first = week[0];
          const prev = weeks[w - 1]?.[0];
          const newMonth = first && (!prev || prev.month !== first.month);
          return (
            <div key={first?.date} className="flex flex-col gap-px">
              <span className="h-3 text-[8px] leading-3 text-muted">
                {newMonth ? monthLabel(first.date) : ""}
              </span>
              {week.map((cell) => (
                <div
                  key={cell.date}
                  title={`${cell.date}: ${cell.plays} plays`}
                  className="h-2.5 w-2.5 border border-transparent hover:border-focus"
                  style={{ background: heatColor(cell.plays / max) }}
                />
              ))}
            </div>
          );
        })}
      </div>
    </div>
  );
}

function HourlyChart({ hourly, peak }: { hourly: number[]; peak: number }) {
  const max = Math.max(1, ...hourly);
  return (
    <div className="flex h-24 items-end gap-px">
      {hourly.map((count, hour) => {
        const isPeak = hour === peak && count > 0;
        return (
          <div
            key={hour}
            title={`${String(hour).padStart(2, "0")}:00 — ${count} plays`}
            className="group flex h-full flex-1 flex-col justify-end gap-1"
          >
            <span
              className={cn(
                "meter-y w-full",
                isPeak ? "bg-accent" : "bg-accent-dim group-hover:bg-accent",
              )}
              style={
                {
                  height: `${Math.max((count / max) * 100, count > 0 ? 3 : 1)}%`,
                  "--i": hour,
                } as React.CSSProperties
              }
            />
            <span
              className={cn(
                "text-center text-[8px] leading-none",
                isPeak ? "text-accent" : "text-muted",
              )}
            >
              {hour % 3 === 0 ? String(hour).padStart(2, "0") : ""}
            </span>
          </div>
        );
      })}
    </div>
  );
}

function WeekdayChart({ weekday, peak }: { weekday: number[]; peak: number }) {
  const max = Math.max(1, ...weekday);
  return (
    <ul className="flex flex-col gap-1">
      {weekday.map((count, i) => (
        <li key={WEEKDAYS[i]} className="flex items-center gap-2">
          <span className="w-7 shrink-0 text-[10px] uppercase text-muted">
            {WEEKDAYS[i]}
          </span>
          <span className="h-2.5 flex-1 bg-raised">
            <span
              className={cn(
                "meter-x block h-full",
                i === peak ? "bg-accent" : "bg-accent-dim",
              )}
              style={
                { width: `${(count / max) * 100}%`, "--i": i } as React.CSSProperties
              }
            />
          </span>
          <span className="w-8 shrink-0 text-right text-[10px] tabular-nums text-muted">
            {count}
          </span>
        </li>
      ))}
    </ul>
  );
}

function TopTracks({ tracks }: { tracks: TrackPlayCount[] }) {
  if (tracks.length === 0) {
    return <p className="text-[11px] text-muted">nothing played yet</p>;
  }
  const max = tracks[0]?.plays ?? 1;
  return (
    <ol className="flex flex-col gap-px">
      {tracks.map((track, i) => (
        <li key={track.trackId}>
          <Link
            to="/albums/$albumId"
            params={{ albumId: String(track.albumId) }}
            title={`${track.title} — ${track.artistName} · ${track.plays} plays`}
            className="group relative flex h-6 items-center gap-2 px-1"
          >
            <span
              className="meter-x absolute inset-y-0 left-0 bg-raised"
              style={
                {
                  width: `${(track.plays / max) * 100}%`,
                  "--i": i,
                } as React.CSSProperties
              }
              aria-hidden
            />
            <span className="relative w-4 shrink-0 text-right text-[10px] tabular-nums text-muted">
              {i + 1}
            </span>
            <span className="relative min-w-0 flex-1 truncate text-[11px] text-primary group-hover:text-accent">
              {track.title}
            </span>
            <span className="relative w-8 shrink-0 text-[10px]">
              {track.favorite && <span className="text-accent">♥</span>}
              {track.rating >= 4 && <span className="text-warn"> ✦</span>}
            </span>
            <span className="relative hidden w-28 shrink-0 truncate text-[10px] text-muted sm:block">
              {track.artistName}
            </span>
            <span className="relative w-8 shrink-0 text-right text-[10px] tabular-nums text-secondary">
              {track.plays}
            </span>
          </Link>
        </li>
      ))}
    </ol>
  );
}

function Meters({ items }: { items: NameCount[] }) {
  if (items.length === 0) {
    return <p className="text-[11px] text-muted">no plays yet</p>;
  }
  const max = items[0]?.count ?? 1;
  return (
    <ul className="flex flex-col gap-1">
      {items.map((item, i) => (
        <li key={item.name} className="flex items-center gap-2">
          <span className="w-28 shrink-0 truncate text-[11px] text-secondary">
            {item.name}
          </span>
          <span className="h-2.5 flex-1 bg-raised">
            <span
              className="meter-x block h-full bg-accent-dim"
              style={
                {
                  width: `${(item.count / max) * 100}%`,
                  "--i": i,
                } as React.CSSProperties
              }
            />
          </span>
          <span className="w-8 shrink-0 text-right text-[10px] tabular-nums text-muted">
            {item.count}
          </span>
        </li>
      ))}
    </ul>
  );
}

function TopAlbums({ albums }: { albums: AlbumPlayCount[] }) {
  const max = albums[0]?.plays ?? 1;
  return (
    <div className="flex flex-wrap gap-3">
      {albums.map((album, i) => (
        <TopAlbumCard key={album.albumId} album={album} rank={i + 1} max={max} />
      ))}
    </div>
  );
}

function TopAlbumCard({
  album,
  rank,
  max,
}: {
  album: AlbumPlayCount;
  rank: number;
  max: number;
}) {
  const [artError, setArtError] = useState(false);
  return (
    <Link
      to="/albums/$albumId"
      params={{ albumId: String(album.albumId) }}
      className="group w-24"
      title={`${album.name} — ${album.artistName} · ${album.plays} plays`}
    >
      <div className="relative aspect-square overflow-hidden border border-subtle bg-raised group-hover:border-focus">
        {!artError ? (
          <img
            src={artworkUrl(album.albumId)}
            alt=""
            loading="lazy"
            onError={() => setArtError(true)}
            className="h-full w-full object-cover"
          />
        ) : (
          <CoverPlaceholder name={album.name} className="text-xl" />
        )}
        <span className="absolute left-0 top-0 bg-base/85 px-1 text-[9px] tabular-nums text-accent">
          {String(rank).padStart(2, "0")}
        </span>
        <span className="absolute inset-x-0 bottom-0 h-0.5 bg-base/60">
          <span
            className="meter-x block h-full bg-accent"
            style={
              { width: `${(album.plays / max) * 100}%`, "--i": rank } as React.CSSProperties
            }
          />
        </span>
      </div>
      <div className="truncate text-[10px] text-secondary group-hover:text-accent">
        {album.name}
      </div>
      <div className="truncate text-[10px] text-muted">{album.plays} plays</div>
    </Link>
  );
}

function FormatPanel({
  codecs,
  losslessPct,
}: {
  codecs: NameCount[];
  losslessPct: number;
}) {
  const max = codecs[0]?.count ?? 1;
  return (
    <div className="flex flex-col gap-3">
      <div>
        <div className="mb-1 flex items-baseline justify-between text-[10px]">
          <span className="uppercase tracking-[0.14em] text-muted">
            lossless share of library
          </span>
          <span className="tabular-nums text-bitperfect">{losslessPct}%</span>
        </div>
        <div className="flex h-3 bg-raised">
          <span
            className="meter-x h-full bg-bitperfect"
            style={{ width: `${losslessPct}%` }}
          />
          <span
            className="meter-x h-full bg-lossy/60"
            style={{ width: `${100 - losslessPct}%`, "--i": 1 } as React.CSSProperties}
          />
        </div>
      </div>
      {codecs.length > 0 && (
        <ul className="flex flex-col gap-1">
          {codecs.map((codec, i) => (
            <li key={codec.name} className="flex items-center gap-2">
              <span
                className={cn(
                  "w-16 shrink-0 truncate text-[11px]",
                  isLossy(codec.name) ? "text-lossy" : "text-bitperfect",
                )}
              >
                {codec.name}
              </span>
              <span className="h-2.5 flex-1 bg-raised">
                <span
                  className={cn(
                    "meter-x block h-full",
                    isLossy(codec.name) ? "bg-lossy/70" : "bg-bitperfect/70",
                  )}
                  style={
                    {
                      width: `${(codec.count / max) * 100}%`,
                      "--i": i,
                    } as React.CSSProperties
                  }
                />
              </span>
              <span className="w-8 shrink-0 text-right text-[10px] tabular-nums text-muted">
                {codec.count}
              </span>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

function LibraryPanel({ library }: { library: LibrarySummary }) {
  const hours = library.totalMs / 3_600_000;
  return (
    <div className="flex flex-col gap-3">
      <dl className="grid grid-cols-2 gap-x-4 gap-y-1 text-[11px] sm:grid-cols-4">
        <Cell label="tracks" value={fmtCount(library.tracks)} />
        <Cell label="albums" value={fmtCount(library.albums)} />
        <Cell label="artists" value={fmtCount(library.artists)} />
        <Cell
          label="runtime"
          value={hours >= 24 ? `${(hours / 24).toFixed(1)}d` : `${hours.toFixed(1)}h`}
        />
      </dl>
      <div className="flex gap-2">
        <Link
          to="/favorites"
          className="group flex flex-1 items-center justify-between border border-subtle px-2 py-1.5 hover:border-focus"
        >
          <span className="text-[11px] text-accent">♥ favorites</span>
          <span className="text-[13px] tabular-nums text-primary group-hover:text-accent">
            {library.favorites}
          </span>
        </Link>
        <Link
          to="/favorites"
          search={{ filter: "liked" }}
          className="group flex flex-1 items-center justify-between border border-subtle px-2 py-1.5 hover:border-focus"
        >
          <span className="text-[11px] text-warn">✦ liked (4★+)</span>
          <span className="text-[13px] tabular-nums text-primary group-hover:text-accent">
            {library.liked}
          </span>
        </Link>
      </div>
    </div>
  );
}

function Cell({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex flex-col">
      <dt className="text-[10px] uppercase tracking-[0.14em] text-muted">
        {label}
      </dt>
      <dd className="text-[13px] tabular-nums text-primary">{value}</dd>
    </div>
  );
}

/** 0 → raised, 1 → full accent; the ramp starts high enough to read at 1 play. */
function heatColor(t: number): string {
  if (t <= 0) return "var(--bg-raised)";
  return `color-mix(in srgb, var(--accent) ${20 + Math.round(t * 80)}%, var(--bg-base))`;
}

function toIsoDay(d: Date): string {
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(
    d.getDate(),
  ).padStart(2, "0")}`;
}

function monthLabel(isoDay: string): string {
  const month = Number(isoDay.slice(5, 7));
  return (
    [
      "jan",
      "feb",
      "mar",
      "apr",
      "may",
      "jun",
      "jul",
      "aug",
      "sep",
      "oct",
      "nov",
      "dec",
    ][month - 1] ?? ""
  );
}

function fmtCount(n: number): string {
  return n.toLocaleString("en-US");
}
