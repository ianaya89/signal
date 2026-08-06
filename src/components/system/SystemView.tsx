import { useNavigate, useRouterState } from "@tanstack/react-router";

import { DoctorView } from "@/components/doctor/DoctorView";
import { LogViewer } from "@/components/logs/LogViewer";
import { StatsView } from "@/components/stats/StatsView";
import { TabbedPane, type PaneTab } from "@/components/ui/TabbedPane";

/** The three views that report on signal itself rather than on your music. */
const TABS: readonly PaneTab[] = [
  {
    key: "stats",
    tone: "var(--sec-stats)",
    blurb: "what you have listened to, and how much of it",
  },
  {
    key: "doctor",
    tone: "var(--sec-doctor)",
    blurb: "missing files, duplicates, and questionable transcodes",
  },
  {
    key: "logs",
    tone: "var(--sec-logs)",
    blurb: "what the app is doing, as it happens",
  },
];

const ROUTES: Record<string, string> = {
  stats: "/stats",
  doctor: "/doctor",
  logs: "/logs",
};

/** True for any path this pane owns — the sidebar uses it to stay lit. */
export function isSystemPath(pathname: string): boolean {
  return (
    pathname === "/system" ||
    Object.values(ROUTES).some((r) => pathname.startsWith(r))
  );
}

/**
 * Stats, doctor and logs share one sidebar entry.
 *
 * Each tab keeps its own route rather than becoming a parameter: they were
 * already reachable from the command palette and the keyboard shortcuts, and
 * turning them into internal state would have broken every one of those links.
 */
export function SystemView() {
  const navigate = useNavigate();
  const pathname = useRouterState({ select: (s) => s.location.pathname });

  const active =
    Object.entries(ROUTES).find(([, route]) =>
      pathname.startsWith(route),
    )?.[0] ?? "stats";

  return (
    <TabbedPane
      tabs={TABS}
      active={active}
      onSelect={(key) => void navigate({ to: ROUTES[key] })}
      label="system sections"
      idPrefix="system"
    >
      {active === "stats" && <StatsView />}
      {active === "doctor" && <DoctorView />}
      {active === "logs" && <LogViewer />}
    </TabbedPane>
  );
}
