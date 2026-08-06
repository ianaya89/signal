import { useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { useEffect, useRef, useState } from "react";

import { useMainTitle } from "@/hooks/useMainTitle";
import { onSignalEvent } from "@/ipc/events";
import { api } from "@/ipc/invoke";
import type {
  AnalysisDoneEvent,
  AnalysisFlaggedTrack,
  AnalysisProgressEvent,
  ArtworkProgressEvent,
} from "@/ipc/types";
import { cn, errText } from "@/lib/utils";
import { toast } from "@/stores/toastStore";

export function DoctorView() {
  useMainTitle("doctor");
  const queryClient = useQueryClient();
  const [fetchingArt, setFetchingArt] = useState(false);
  const [artLog, setArtLog] = useState<ArtworkProgressEvent[]>([]);
  const [artProgress, setArtProgress] = useState<{
    processed: number;
    total: number;
  } | null>(null);
  const logRef = useRef<HTMLUListElement>(null);
  const { data, isLoading, refetch, isFetching } = useQuery({
    queryKey: ["health"],
    queryFn: api.libraryHealth,
    staleTime: 60_000,
  });

  // null = trust the backend's `running` flag (restores a live run on mount)
  const [analyzing, setAnalyzing] = useState<boolean | null>(null);
  const [analysisLog, setAnalysisLog] = useState<AnalysisProgressEvent[]>([]);
  const [analysisProgress, setAnalysisProgress] = useState<{
    processed: number;
    total: number;
  } | null>(null);
  const analysisLogRef = useRef<HTMLUListElement>(null);
  const { data: analysis } = useQuery({
    queryKey: ["analysis"],
    queryFn: api.analysisReport,
    staleTime: 15_000,
  });
  const analysisRunning = analyzing ?? analysis?.running ?? false;

  useEffect(() => {
    const unlisten = onSignalEvent<ArtworkProgressEvent>(
      "artwork:progress",
      (payload) => {
        setArtProgress({ processed: payload.processed, total: payload.total });
        setArtLog((log) => [...log, payload]);
        requestAnimationFrame(() => {
          const el = logRef.current;
          if (el) el.scrollTop = el.scrollHeight;
        });
      },
    );
    return () => void unlisten.then((fn) => fn());
  }, []);

  useEffect(() => {
    const unlistenProgress = onSignalEvent<AnalysisProgressEvent>(
      "analysis:progress",
      (payload) => {
        setAnalysisProgress({ processed: payload.processed, total: payload.total });
        // libraries run 10k+ tracks — keep only the tail of the log
        setAnalysisLog((log) => [...log, payload].slice(-500));
        requestAnimationFrame(() => {
          const el = analysisLogRef.current;
          if (el) el.scrollTop = el.scrollHeight;
        });
      },
    );
    const unlistenDone = onSignalEvent<AnalysisDoneEvent>("analysis:done", (done) => {
      setAnalyzing(false);
      setAnalysisProgress(null);
      toast.ok(
        done.cancelled
          ? "analysis stopped"
          : done.flagged > 0
            ? `${done.flagged} suspicious files found`
            : `${done.analyzed} files analyzed — all clean`,
      );
    });
    return () => {
      void unlistenProgress.then((fn) => fn());
      void unlistenDone.then((fn) => fn());
    };
  }, []);

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

  const fetchArt = async () => {
    setFetchingArt(true);
    setArtLog([]);
    setArtProgress({ processed: 0, total: Math.min(data.albumsWithoutArtTotal, 15) });
    try {
      const result = await api.fetchArtwork();
      toast.ok(
        result.fetched > 0
          ? `${result.fetched} covers fetched · ${result.remaining} albums still bare`
          : result.cancelled
            ? "artwork lookup stopped"
            : "no covers matched this batch",
      );
      await queryClient.invalidateQueries();
    } catch (err) {
      toast.error(errText(err));
      setArtLog((log) => [
        ...log,
        {
          processed: 0,
          total: 0,
          album: "lookup failed",
          artist: "",
          outcome: "error",
          detail: errText(err),
        },
      ]);
    } finally {
      setFetchingArt(false);
      setArtProgress(null);
    }
  };

  const startAnalysis = async (force: boolean) => {
    setAnalysisLog([]);
    setAnalysisProgress(null);
    setAnalyzing(true);
    try {
      const queued = await api.analysisStart(force);
      if (queued === 0) {
        setAnalyzing(false);
        toast.info("nothing new to analyze — use re-analyze all");
      }
    } catch (err) {
      setAnalyzing(false);
      toast.error(errText(err));
    }
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
        action={
          data.albumsWithoutArtTotal > 0 ? (
            <span className="flex items-center gap-1.5">
              <button
                type="button"
                onClick={() => void fetchArt()}
                disabled={fetchingArt}
                title="look up covers on MusicBrainz + Cover Art Archive — 15 albums per run, ~1s each (their rate limit)"
                className="border border-subtle px-2 py-0.5 text-[11px] text-accent hover:border-focus disabled:opacity-40"
              >
                {fetchingArt ? "looking up…" : "fetch online…"}
              </button>
              {fetchingArt && (
                <button
                  type="button"
                  onClick={() => void api.fetchArtworkCancel()}
                  className="border border-subtle px-2 py-0.5 text-[11px] text-muted hover:border-error hover:text-error"
                >
                  stop
                </button>
              )}
            </span>
          ) : undefined
        }
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

      {(fetchingArt || artLog.length > 0) && (
        <ArtworkRun
          logRef={logRef}
          log={artLog}
          progress={artProgress}
          running={fetchingArt}
          onDismiss={() => setArtLog([])}
        />
      )}

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

      <Issue
        title="suspicious audio"
        count={analysis?.flagged.length ?? 0}
        ok={
          (analysis?.summary.analyzedTotal ?? 0) > 0
            ? `no fake hi-res or transcodes among ${analysis?.summary.analyzedTotal} analyzed files`
            : "not analyzed yet"
        }
        hint="spectral analysis of lossless files"
        action={
          <span className="flex items-center gap-1.5">
            <button
              type="button"
              onClick={() =>
                void startAnalysis((analysis?.summary.analyzedTotal ?? 0) > 0)
              }
              disabled={analysisRunning}
              title="decode lossless files and inspect their spectrum for upsampling, lossy transcodes and padded bit depth"
              className="border border-subtle px-2 py-0.5 text-[11px] text-accent hover:border-focus disabled:opacity-40"
            >
              {analysisRunning
                ? "analyzing…"
                : (analysis?.summary.analyzedTotal ?? 0) > 0
                  ? "re-analyze all"
                  : "analyze library"}
            </button>
            {analysisRunning && (
              <button
                type="button"
                onClick={() => void api.analysisCancel()}
                className="border border-subtle px-2 py-0.5 text-[11px] text-muted hover:border-error hover:text-error"
              >
                stop
              </button>
            )}
          </span>
        }
      >
        {(analysis?.flagged ?? []).map((t) => (
          <li key={t.id} className="flex min-w-0 items-baseline gap-2">
            <span
              className={cn(
                "shrink-0",
                t.verdict === "padded_bits" ? "text-muted" : "text-warn",
              )}
            >
              {verdictLabel(t.verdict)}
            </span>
            <Link
              to="/albums/$albumId"
              params={{ albumId: String(t.albumId) }}
              className="shrink-0 text-secondary hover:text-accent"
            >
              {t.artistName} — {t.title}
            </Link>
            <span className="min-w-0 truncate text-muted">{t.detail}</span>
            <span className="ml-auto shrink-0 tabular-nums text-[10px] text-muted">
              {Math.round(t.confidence * 100)}%
            </span>
          </li>
        ))}
      </Issue>

      {(analysisRunning || analysisLog.length > 0) && (
        <AnalysisRun
          logRef={analysisLogRef}
          log={analysisLog}
          progress={analysisProgress}
          running={analysisRunning}
          onDismiss={() => setAnalysisLog([])}
        />
      )}

      <div className="flex gap-6 text-[11px] text-muted">
        <span>{data.tracksWithoutYear} tracks without year</span>
        <span>{data.tracksWithoutGenre} tracks without genre</span>
      </div>
    </div>
  );
}

/** Live console for the artwork lookup: the run is minutes long by protocol,
 *  so every album's verdict lands here as it resolves. */
function ArtworkRun({
  logRef,
  log,
  progress,
  running,
  onDismiss,
}: {
  logRef: React.RefObject<HTMLUListElement | null>;
  log: ArtworkProgressEvent[];
  progress: { processed: number; total: number } | null;
  running: boolean;
  onDismiss: () => void;
}) {
  const found = log.filter((l) => l.outcome === "found").length;
  const done = progress?.processed ?? log.length;
  const total = progress?.total ?? log.length;
  const remainingSecs = Math.max(total - done, 0) * 2;

  return (
    <section className="border border-subtle bg-surface">
      <header className="flex h-7 items-center gap-2 border-b border-subtle px-2 text-[10px]">
        <span className="uppercase tracking-[0.14em] text-accent">
          [ artwork lookup ]
        </span>
        <span className="tabular-nums text-muted">
          {done}/{total}
        </span>
        <span className="text-ok">{found} found</span>
        {running && remainingSecs > 0 && (
          <span className="text-muted">· ~{remainingSecs}s left</span>
        )}
        {!running && (
          <button
            type="button"
            onClick={onDismiss}
            className="ml-auto text-[11px] text-muted hover:text-accent"
          >
            dismiss
          </button>
        )}
      </header>
      <div className="h-1 bg-base">
        <div
          className="h-full bg-accent transition-[width] duration-300"
          style={{ width: total > 0 ? `${(done / total) * 100}%` : "0%" }}
        />
      </div>
      <ul
        ref={logRef}
        className="flex max-h-40 flex-col gap-0.5 overflow-auto px-2 py-1.5 text-[11px]"
      >
        {log.map((entry, i) => (
          <li key={`${entry.album}-${i}`} className="flex min-w-0 gap-2">
            <span
              className={cn(
                "w-3 shrink-0",
                entry.outcome === "found"
                  ? "text-ok"
                  : entry.outcome === "error"
                    ? "text-error"
                    : "text-muted",
              )}
            >
              {entry.outcome === "found" ? "✓" : entry.outcome === "error" ? "✕" : "—"}
            </span>
            <span className="shrink-0 text-secondary">{entry.album}</span>
            {entry.artist && (
              <span className="shrink-0 text-muted">· {entry.artist}</span>
            )}
            {entry.detail && (
              <span
                className={cn(
                  "min-w-0 truncate",
                  entry.outcome === "error" ? "text-error" : "text-muted",
                )}
              >
                {entry.detail}
              </span>
            )}
          </li>
        ))}
        {running && (
          <li className="text-muted">
            querying musicbrainz… (their rate limit is 1 request/second)
          </li>
        )}
      </ul>
    </section>
  );
}

function verdictLabel(verdict: AnalysisFlaggedTrack["verdict"]): string {
  return verdict === "upsampled"
    ? "fake hi-res"
    : verdict === "transcode"
      ? "lossy transcode"
      : "padded bits";
}

/** Live console for the audio authenticity analysis, mirroring ArtworkRun:
 *  a library-wide run takes minutes, so every verdict streams in here. */
function AnalysisRun({
  logRef,
  log,
  progress,
  running,
  onDismiss,
}: {
  logRef: React.RefObject<HTMLUListElement | null>;
  log: AnalysisProgressEvent[];
  progress: { processed: number; total: number } | null;
  running: boolean;
  onDismiss: () => void;
}) {
  const flagged = log.filter(
    (l) => l.verdict === "upsampled" || l.verdict === "transcode" || l.verdict === "padded_bits",
  ).length;
  const done = progress?.processed ?? log.length;
  const total = progress?.total ?? log.length;

  return (
    <section className="border border-subtle bg-surface">
      <header className="flex h-7 items-center gap-2 border-b border-subtle px-2 text-[10px]">
        <span className="uppercase tracking-[0.14em] text-accent">
          [ audio analysis ]
        </span>
        <span className="tabular-nums text-muted">
          {done}/{total}
        </span>
        <span className={flagged > 0 ? "text-warn" : "text-ok"}>
          {flagged} flagged
        </span>
        {!running && (
          <button
            type="button"
            onClick={onDismiss}
            className="ml-auto text-[11px] text-muted hover:text-accent"
          >
            dismiss
          </button>
        )}
      </header>
      <div className="h-1 bg-base">
        <div
          className="h-full bg-accent transition-[width] duration-300"
          style={{ width: total > 0 ? `${(done / total) * 100}%` : "0%" }}
        />
      </div>
      <ul
        ref={logRef}
        className="flex max-h-40 flex-col gap-0.5 overflow-auto px-2 py-1.5 text-[11px]"
      >
        {log.map((entry, i) => (
          <li key={`${entry.trackId}-${i}`} className="flex min-w-0 gap-2">
            <span
              className={cn(
                "w-3 shrink-0",
                entry.verdict === "clean"
                  ? "text-ok"
                  : entry.verdict === "unreadable"
                    ? "text-error"
                    : entry.verdict === "skipped"
                      ? "text-muted"
                      : "text-warn",
              )}
            >
              {entry.verdict === "clean"
                ? "✓"
                : entry.verdict === "unreadable"
                  ? "✕"
                  : entry.verdict === "skipped"
                    ? "—"
                    : "!"}
            </span>
            <span className="shrink-0 text-secondary">{entry.title}</span>
            {entry.artist && (
              <span className="shrink-0 text-muted">· {entry.artist}</span>
            )}
            {entry.detail && (
              <span
                className={cn(
                  "min-w-0 truncate",
                  entry.verdict === "unreadable" ? "text-error" : "text-muted",
                )}
              >
                {entry.detail}
              </span>
            )}
          </li>
        ))}
        {running && <li className="text-muted">decoding spectra…</li>}
      </ul>
    </section>
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
