import { useQuery } from "@tanstack/react-query";
import { useEffect } from "react";

import { api } from "@/ipc/invoke";
import { checkForUpdate, installUpdate, restartNow } from "@/lib/updater";
import { cn } from "@/lib/utils";
import { useUpdateStore } from "@/stores/updateStore";

/** Review step between "an update exists" and a multi-minute download:
 *  version, notes, live progress, and errors that stay on screen. */
export function UpdateDialog() {
  const open = useUpdateStore((s) => s.dialogOpen);
  const close = useUpdateStore((s) => s.closeDialog);
  const status = useUpdateStore((s) => s.status);
  const version = useUpdateStore((s) => s.version);
  const notes = useUpdateStore((s) => s.notes);
  const downloaded = useUpdateStore((s) => s.downloaded);
  const total = useUpdateStore((s) => s.total);
  const error = useUpdateStore((s) => s.error);

  const { data: info } = useQuery({
    queryKey: ["app-info"],
    queryFn: api.appInfo,
    staleTime: Infinity,
  });

  useEffect(() => {
    if (!open) return;
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        close();
      } else if (e.key === "Enter" && status === "available") {
        e.preventDefault();
        void installUpdate();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [open, close, status]);

  if (!open) return null;

  const pct =
    total && total > 0 ? Math.min(Math.round((downloaded / total) * 100), 100) : null;

  return (
    <div
      className="absolute inset-0 z-[60] flex items-center justify-center bg-black/50"
      onMouseDown={close}
    >
      <div
        className="flex max-h-[70vh] w-[520px] flex-col border border-focus bg-raised"
        onMouseDown={(e) => e.stopPropagation()}
      >
        <header className="flex h-8 shrink-0 items-center justify-between border-b border-subtle px-3">
          <span className="text-[12px] text-accent">[ software update ]</span>
          <span className="text-[10px] text-muted">esc to close</span>
        </header>

        <div className="flex shrink-0 items-baseline gap-2 border-b border-subtle px-3 py-2 text-[12px]">
          <span className="text-muted">v{info?.version ?? "—"}</span>
          <span className="text-muted">→</span>
          <span className="text-[15px] text-accent">
            v{version ?? (status === "checking" ? "…" : "—")}
          </span>
          <StatusChip status={status} pct={pct} />
        </div>

        <div className="min-h-0 flex-1 overflow-auto px-3 py-2">
          {status === "checking" && (
            <p className="text-[11px] text-muted">checking for updates…</p>
          )}
          {status === "idle" && (
            <p className="text-[11px] text-muted">
              signal is up to date — nothing to install
            </p>
          )}
          {notes ? (
            <pre className="whitespace-pre-wrap text-[11px] leading-relaxed text-secondary">
              {notes.trim()}
            </pre>
          ) : status === "available" ? (
            <p className="text-[11px] text-muted">
              no release notes published for this version
            </p>
          ) : null}
          {error && (
            <p className="mt-2 border border-error px-2 py-1 text-[11px] text-error">
              {error}
            </p>
          )}
        </div>

        {status === "downloading" && (
          <div className="shrink-0 border-t border-subtle px-3 py-2">
            <div className="mb-1 flex justify-between text-[10px] text-muted">
              <span>{pct === null ? "downloading…" : `downloading ${pct}%`}</span>
              <span className="tabular-nums">
                {fmtBytes(downloaded)}
                {total ? ` / ${fmtBytes(total)}` : ""}
              </span>
            </div>
            <div className="h-1.5 bg-base">
              <div
                className={cn(
                  "h-full bg-accent",
                  pct === null && "w-1/3 animate-pulse",
                )}
                style={pct === null ? undefined : { width: `${pct}%` }}
              />
            </div>
          </div>
        )}

        <footer className="flex shrink-0 items-center justify-end gap-2 border-t border-subtle px-3 py-2">
          <button
            type="button"
            onClick={close}
            className="border border-subtle px-2 py-0.5 text-[11px] text-muted hover:border-focus hover:text-secondary"
          >
            {status === "downloading" ? "hide" : "later"}
          </button>
          {status === "ready" ? (
            <button
              type="button"
              onClick={() => void restartNow()}
              className="border border-focus bg-surface px-2 py-0.5 text-[11px] text-accent hover:bg-base"
            >
              restart now
            </button>
          ) : status === "downloading" ? (
            <button
              type="button"
              disabled
              className="border border-subtle px-2 py-0.5 text-[11px] text-muted opacity-50"
            >
              installing…
            </button>
          ) : status === "available" ? (
            <button
              type="button"
              onClick={() => void installUpdate()}
              className="border border-focus bg-surface px-2 py-0.5 text-[11px] text-accent hover:bg-base"
            >
              install + restart ⏎
            </button>
          ) : (
            <button
              type="button"
              onClick={() => void checkForUpdate()}
              disabled={status === "checking"}
              className="border border-subtle px-2 py-0.5 text-[11px] text-secondary hover:border-focus hover:text-accent disabled:opacity-50"
            >
              check again
            </button>
          )}
        </footer>
      </div>
    </div>
  );
}

function StatusChip({
  status,
  pct,
}: {
  status: string;
  pct: number | null;
}) {
  const label =
    status === "downloading"
      ? pct === null
        ? "downloading"
        : `downloading ${pct}%`
      : status === "ready"
        ? "installed"
        : status === "error"
          ? "failed"
          : status === "available"
            ? "ready to install"
            : null;
  if (!label) return null;
  return (
    <span
      className={cn(
        "ml-auto px-1.5 text-[10px] uppercase tracking-[0.12em]",
        status === "error" ? "text-error" : "text-muted",
      )}
    >
      {label}
    </span>
  );
}

function fmtBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(0)} KB`;
  return `${(n / (1024 * 1024)).toFixed(1)} MB`;
}
