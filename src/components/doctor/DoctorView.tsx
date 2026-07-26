import { useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";

import { useMainTitle } from "@/hooks/useMainTitle";
import { api } from "@/ipc/invoke";
import { cn } from "@/lib/utils";
import { toast } from "@/stores/toastStore";

export function DoctorView() {
  useMainTitle("doctor");
  const queryClient = useQueryClient();
  const { data, isLoading, refetch, isFetching } = useQuery({
    queryKey: ["health"],
    queryFn: api.libraryHealth,
    staleTime: 60_000,
  });

  if (isLoading || !data) {
    return <p className="p-3 text-muted">examining library…</p>;
  }

  const prune = async () => {
    const ids = data.missingFiles.map((t) => t.id);
    if (ids.length === 0) return;
    const removed = await api.libraryPruneMissing(ids);
    toast.ok(`${removed} dead entries removed`);
    await queryClient.invalidateQueries();
  };

  const relink = async () => {
    const relinked = await api.libraryRelinkMissing();
    toast.ok(
      relinked > 0
        ? `${relinked} moved files re-linked`
        : "no moved files matched — rescan first so new locations are imported",
    );
    await queryClient.invalidateQueries();
  };

  const resolveDupes = async () => {
    const merged = await api.libraryResolveDuplicates();
    toast.ok(merged > 0 ? `${merged} duplicates merged into best copy` : "nothing to merge");
    await queryClient.invalidateQueries();
  };

  return (
    <div className="flex flex-col gap-4 p-4">
      <header className="flex items-center gap-6">
        <ScoreRing score={data.score} />
        <div className="flex flex-col gap-0.5 text-[12px]">
          <span className="text-primary">
            {data.totalTracks} tracks · {data.losslessPct}% lossless
          </span>
          <span className="text-muted">
            library health {label(data.score)}
          </span>
          <button
            type="button"
            onClick={() => void refetch()}
            className="mt-1 w-fit border border-subtle bg-raised px-2 py-0.5 text-[11px] text-secondary hover:border-focus hover:text-accent"
          >
            {isFetching ? "examining…" : "re-examine"}
          </button>
        </div>
      </header>

      <Issue
        title="dead files"
        count={data.missingFilesTotal}
        ok="every file on disk"
        action={
          data.missingFilesTotal > 0 ? (
            <span className="flex gap-1.5">
              <button
                type="button"
                onClick={() => void relink()}
                title="match dead entries to moved files by content hash — stats and playlists survive"
                className="border border-subtle px-2 py-0.5 text-[11px] text-accent hover:border-focus"
              >
                relink moved files
              </button>
              <button
                type="button"
                onClick={() => void prune()}
                className="border border-subtle px-2 py-0.5 text-[11px] text-error hover:border-error"
              >
                remove {data.missingFiles.length} from library
              </button>
            </span>
          ) : undefined
        }
      >
        {data.missingFiles.map((t) => (
          <li key={t.id} className="flex min-w-0 gap-2">
            <span className="shrink-0 text-secondary">{t.title}</span>
            <span className="min-w-0 truncate text-muted">{t.detail}</span>
          </li>
        ))}
      </Issue>

      <Issue
        title="possible duplicates"
        count={data.duplicatesTotal}
        ok="no duplicates detected"
        hint="same artist + title + similar duration"
        action={
          data.duplicatesTotal > 0 ? (
            <button
              type="button"
              onClick={() => void resolveDupes()}
              title="keep the best-quality copy of each group, merge stats and playlists into it (db only — files stay)"
              className="border border-subtle px-2 py-0.5 text-[11px] text-accent hover:border-focus"
            >
              resolve all · keep best
            </button>
          ) : undefined
        }
      >
        {data.duplicates.map((d) => (
          <li key={`${d.artistName}-${d.title}`} className="flex gap-2">
            <span className="text-secondary">
              {d.artistName} — {d.title}
            </span>
            <span className="text-warn">×{d.count}</span>
          </li>
        ))}
      </Issue>

      <Issue
        title="albums without artwork"
        count={data.albumsWithoutArtTotal}
        ok="every album has artwork"
      >
        {data.albumsWithoutArt.map((a) => (
          <li key={a.id}>
            <Link
              to="/albums/$albumId"
              params={{ albumId: String(a.id) }}
              className="text-secondary hover:text-accent"
            >
              {a.artistName} — {a.name}
            </Link>
            <span className="ml-2 text-[10px] text-muted">
              (open to set artwork)
            </span>
          </li>
        ))}
      </Issue>

      <Issue
        title="low-bitrate lossy"
        count={data.lowBitrateTotal}
        ok="no suspicious rips"
        hint="lossy under 160 kbps"
      >
        {data.lowBitrate.map((t) => (
          <li key={t.id} className="flex gap-2">
            <span className="text-secondary">
              {t.artistName} — {t.title}
            </span>
            <span className="text-warn">{t.detail}</span>
          </li>
        ))}
      </Issue>

      <div className="flex gap-6 text-[11px] text-muted">
        <span>{data.tracksWithoutYear} tracks without year</span>
        <span>{data.tracksWithoutGenre} tracks without genre</span>
      </div>
    </div>
  );
}

function label(score: number): string {
  if (score >= 90) return "excellent";
  if (score >= 75) return "good";
  if (score >= 50) return "needs attention";
  return "critical";
}

function ScoreRing({ score }: { score: number }) {
  const color =
    score >= 90 ? "var(--ok)" : score >= 75 ? "var(--accent)" : score >= 50 ? "var(--warn)" : "var(--error)";
  const r = 26;
  const c = 2 * Math.PI * r;
  return (
    <svg width="72" height="72" viewBox="0 0 72 72" aria-hidden>
      <circle cx="36" cy="36" r={r} fill="none" stroke="var(--bg-raised)" strokeWidth="6" />
      <circle
        cx="36"
        cy="36"
        r={r}
        fill="none"
        stroke={color}
        strokeWidth="6"
        strokeDasharray={`${(score / 100) * c} ${c}`}
        transform="rotate(-90 36 36)"
      />
      <text
        x="36"
        y="40"
        textAnchor="middle"
        fill="var(--text-primary)"
        fontSize="16"
        fontFamily="var(--font-mono)"
      >
        {score}
      </text>
    </svg>
  );
}

function Issue({
  title,
  count,
  ok,
  hint,
  action,
  children,
}: {
  title: string;
  count: number;
  ok: string;
  hint?: string;
  action?: React.ReactNode;
  children: React.ReactNode;
}) {
  return (
    <section>
      <div className="mb-1 flex items-center gap-2">
        <h2 className="text-[10px] uppercase tracking-wider text-muted">{title}</h2>
        <span
          className={cn(
            "px-1 text-[10px]",
            count === 0 ? "text-ok" : "bg-raised text-warn",
          )}
        >
          {count === 0 ? "✓" : count}
        </span>
        {hint && <span className="text-[10px] text-muted">· {hint}</span>}
        {action}
      </div>
      {count === 0 ? (
        <p className="text-[11px] text-muted">{ok}</p>
      ) : (
        <ul className="flex max-h-44 flex-col gap-0.5 overflow-auto text-[11px]">
          {children}
        </ul>
      )}
    </section>
  );
}
