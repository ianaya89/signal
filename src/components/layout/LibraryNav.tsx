import { Link, useRouterState } from "@tanstack/react-router";

import { isSystemPath } from "@/components/system/SystemView";
import { cn } from "@/lib/utils";

const SECTIONS = [
  { label: "albums", to: "/", exact: true },
  { label: "artists", to: "/artists", exact: false },
  { label: "favorites", to: "/favorites", exact: false },
  { label: "genres", to: "/genres", exact: false },
  { label: "folders", to: "/folders", exact: false },
  { label: "remote", to: "/remote", exact: false },
  { label: "playlists", to: "/playlists", exact: false },
  { label: "discover", to: "/discover", exact: false },
  { label: "system", to: "/system", exact: false },
  { label: "settings", to: "/settings", exact: false },
] as const;

export function LibraryNav() {
  const pathname = useRouterState({ select: (s) => s.location.pathname });

  return (
    <nav className="flex flex-col gap-px p-1.5">
      {SECTIONS.map((s) => {
        // system owns three paths that don't share its prefix, so it can't
        // rely on startsWith the way the others do
        const active =
          s.to === "/system"
            ? isSystemPath(pathname)
            : s.exact
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
    </nav>
  );
}
