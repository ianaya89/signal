import { useQuery } from "@tanstack/react-query";

import { TrackRow } from "@/components/library/TrackRow";
import { useMainTitle } from "@/hooks/useMainTitle";
import { api } from "@/ipc/invoke";
import type { Track } from "@/ipc/types";

/** Recommendation shelves computed from listening signal — plain SQL over
 *  ratings, favorites, play counts and recent history. */
export function DiscoverView() {
  useMainTitle("discover");
  const { data, isLoading } = useQuery({
    queryKey: ["discover"],
    queryFn: api.discover,
    staleTime: 60_000,
  });

  if (isLoading || !data) {
    return <p className="p-3 text-muted">reading your signal…</p>;
  }

  const empty =
    data.onRepeat.length === 0 &&
    data.rediscover.length === 0 &&
    data.fromYourArtists.length === 0 &&
    data.neverPlayed.length === 0;
  if (empty) {
    return (
      <p className="p-3 text-[12px] text-muted">
        nothing to recommend yet — play some music, ✦ what you like
      </p>
    );
  }

  return (
    <div className="flex flex-col gap-5 p-3">
      <Shelf
        title="on repeat"
        hint="most played, last 30 days"
        tracks={data.onRepeat}
      />
      <Shelf
        title="rediscover"
        hint="loved but not heard lately"
        tracks={data.rediscover}
      />
      <Shelf
        title="from your artists"
        hint="unheard tracks by artists you play"
        tracks={data.fromYourArtists}
      />
      <Shelf
        title="never played"
        hint="random unplayed corners"
        tracks={data.neverPlayed}
      />
    </div>
  );
}

function Shelf({
  title,
  hint,
  tracks,
}: {
  title: string;
  hint: string;
  tracks: Track[];
}) {
  if (tracks.length === 0) return null;
  const ids = tracks.map((t) => t.id);
  return (
    <section>
      <div className="mb-1 flex items-baseline gap-2">
        <h2 className="text-[10px] uppercase tracking-wider text-accent">
          {title}
        </h2>
        <span className="text-[10px] text-muted">· {hint}</span>
      </div>
      <table className="w-full border-collapse">
        <tbody>
          {tracks.map((track, i) => (
            <TrackRow
              key={track.id}
              track={track}
              onPlay={() => void api.playContext(ids, i)}
            />
          ))}
        </tbody>
      </table>
    </section>
  );
}
