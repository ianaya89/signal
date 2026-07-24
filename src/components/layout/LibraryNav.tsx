import { Link, useRouterState } from "@tanstack/react-router";

import { cn } from "@/lib/utils";

const SECTIONS = [
  { label: "albums", to: "/", exact: true },
  { label: "artists", to: "/artists", exact: false },
  { label: "playlists", to: "/playlists", exact: false },
  { label: "stats", to: "/stats", exact: false },
  { label: "logs", to: "/logs", exact: false },
] as const;

const PLANNED = [
  { label: "genres", milestone: "soon" },
  { label: "folders", milestone: "soon" },
] as const;

export function LibraryNav() {
  const pathname = useRouterState({ select: (s) => s.location.pathname });

  return (
    <nav className="flex flex-col gap-px p-1.5">
      {SECTIONS.map((s) => {
        const active = s.exact
          ? pathname === s.to || pathname.startsWith("/albums")
          : pathname.startsWith(s.to);
        return (
          <Link
            key={s.label}
            to={s.to}
            className={cn(
              "flex h-7 items-center justify-between rounded-[var(--radius-sm)] px-2 hover:bg-raised",
              active ? "bg-raised" : undefined,
            )}
          >
            <span className={active ? "text-accent" : "text-secondary"}>
              {s.label}
            </span>
            {active && <span className="text-[9px] text-accent">●</span>}
          </Link>
        );
      })}
      {PLANNED.map((s) => (
        <div
          key={s.label}
          className="flex h-7 cursor-default items-center justify-between px-2 opacity-60"
        >
          <span className="text-muted">{s.label}</span>
          <span className="rounded-[var(--radius-sm)] bg-raised px-1 text-[10px] text-muted">
            {s.milestone}
          </span>
        </div>
      ))}
    </nav>
  );
}
