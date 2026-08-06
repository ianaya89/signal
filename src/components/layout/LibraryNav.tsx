import { Link, useRouterState } from "@tanstack/react-router";

import { isSystemPath } from "@/components/system/SystemView";
import { cn } from "@/lib/utils";

// Hues match the panes they open, so the sidebar and the tab strips read as
// one system. Only the active row is tinted — ten lit rows would be noise.
const SECTIONS = [
  { label: "albums", to: "/", exact: true, tone: "var(--sec-library)" },
  { label: "artists", to: "/artists", exact: false, tone: "var(--sec-library)" },
  { label: "favorites", to: "/favorites", exact: false, tone: "var(--error)" },
  { label: "genres", to: "/genres", exact: false, tone: "var(--sec-appearance)" },
  { label: "folders", to: "/folders", exact: false, tone: "var(--sec-library)" },
  { label: "remote", to: "/remote", exact: false, tone: "var(--sec-remote)" },
  { label: "playlists", to: "/playlists", exact: false, tone: "var(--sec-scrobbling)" },
  { label: "discover", to: "/discover", exact: false, tone: "var(--sec-playback)" },
  { label: "system", to: "/system", exact: false, tone: "var(--sec-stats)" },
  { label: "settings", to: "/settings", exact: false, tone: "var(--sec-server)" },
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
            style={{ "--tone": s.tone } as React.CSSProperties}
            className={cn(
              "flex h-7 items-center justify-between border-l-2 px-2 transition-colors hover:bg-raised",
              active
                ? "border-l-[color:var(--tone)] bg-raised"
                : "border-l-transparent hover:border-l-[color-mix(in_srgb,var(--tone)_50%,transparent)]",
            )}
          >
            <span
              className={active ? "text-[color:var(--tone)]" : "text-secondary"}
            >
              {s.label}
            </span>
            {active && (
              <span className="led text-[7px] leading-none text-[color:var(--tone)]">
                ●
              </span>
            )}
          </Link>
        );
      })}
    </nav>
  );
}
