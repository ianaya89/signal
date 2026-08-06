import { useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";

import { Loading } from "@/components/ui/States";
import { useMainTitle } from "@/hooks/useMainTitle";
import { api } from "@/ipc/invoke";
import { cn } from "@/lib/utils";

export function RemoteSourcesView() {
  useMainTitle("remote");
  const { data: sources, isLoading } = useQuery({
    queryKey: ["remote-sources"],
    queryFn: api.remoteSourceList,
  });

  if (isLoading) return <Loading />;

  if (!sources || sources.length === 0) {
    return (
      <div className="flex flex-col gap-2 p-4">
        <p className="text-[12px] text-secondary">no remote servers yet</p>
        <p className="text-[11px] text-muted">
          add a navidrome/airsonic/gonic server in{" "}
          <Link to="/settings" className="text-accent hover:underline">
            settings
          </Link>{" "}
          to browse and stream from it.
        </p>
      </div>
    );
  }

  return (
    <div className="py-1">
      {sources.map((source) => (
        <Link
          key={source.id}
          to="/remote/$sourceId"
          params={{ sourceId: String(source.id) }}
          className="group flex h-7 items-center gap-3 border-l-2 border-transparent px-3 hover:border-focus hover:bg-raised"
        >
          <span className="min-w-0 flex-1 truncate text-[12px] text-primary group-hover:text-accent">
            {source.name}
          </span>
          <span className="truncate text-[10px] text-muted">
            {source.baseUrl}
          </span>
          <span
            className={cn(
              "text-[9px]",
              source.lastPingOk === true
                ? "text-ok"
                : source.lastPingOk === false
                  ? "text-error"
                  : "text-muted",
            )}
          >
            ●
          </span>
        </Link>
      ))}
    </div>
  );
}
