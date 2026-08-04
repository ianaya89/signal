import { useQuery } from "@tanstack/react-query";
import { useState } from "react";

import { CoverPlaceholder } from "@/components/ui/CoverPlaceholder";
import { api } from "@/ipc/invoke";
import { cn } from "@/lib/utils";

/** Remote artwork. The URL has to be built in Rust because it carries the
 *  per-request auth token, so it arrives through a (cached) query rather than
 *  being derived inline the way local `artworkUrl` is. */
export function RemoteCover({
  sourceId,
  coverArt,
  name,
  size = 96,
  className,
}: {
  sourceId: number;
  coverArt: string | undefined;
  name: string;
  size?: number;
  className?: string;
}) {
  const [failed, setFailed] = useState(false);
  const { data: url } = useQuery({
    queryKey: ["remote-cover", sourceId, coverArt, size],
    queryFn: () => api.remoteCoverArtUrl(sourceId, coverArt ?? "", size),
    enabled: coverArt != null,
    staleTime: Infinity,
  });

  return (
    <div
      className={cn(
        "shrink-0 overflow-hidden border border-subtle bg-raised",
        className,
      )}
    >
      {url && !failed ? (
        <img
          src={url}
          alt=""
          loading="lazy"
          onError={() => setFailed(true)}
          className="h-full w-full object-cover"
        />
      ) : (
        <CoverPlaceholder name={name} showName={false} />
      )}
    </div>
  );
}
