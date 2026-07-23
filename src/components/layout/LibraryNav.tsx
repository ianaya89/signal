import { Link, useRouterState } from "@tanstack/react-router";

import { cn } from "@/lib/utils";

const SECTIONS = [
  { label: "albums", to: "/", exact: true },
  { label: "artists", to: "/artists", exact: false },
  { label: "stats", to: "/stats", exact: false },
] as const;

const PLANNED = [
  { label: "genres", milestone: "M3" },
  { label: "folders", milestone: "M4" },
] as const;

export function LibraryNav() {
  const pathname = useRouterState({ select: (s) => s.location.pathname });

  return (
    <nav className="py-1">
      {SECTIONS.map((s) => {
        const active = s.exact
          ? pathname === s.to || pathname.startsWith("/albums")
          : pathname.startsWith(s.to);
        return (
          <Link
            key={s.label}
            to={s.to}
            className={cn(
              "flex h-7 items-center justify-between px-2 hover:bg-raised",
              active ? "bg-raised" : undefined,
            )}
          >
            <span className={active ? "text-accent" : "text-secondary"}>
              {s.label}
            </span>
          </Link>
        );
      })}
      {PLANNED.map((s) => (
        <div
          key={s.label}
          className="flex h-7 cursor-default items-center justify-between px-2"
        >
          <span className="text-muted">{s.label}</span>
          <span className="text-[10px] text-muted">{s.milestone}</span>
        </div>
      ))}
    </nav>
  );
}
