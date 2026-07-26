import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef, useState } from "react";

import { api } from "@/ipc/invoke";
import { artworkUrl } from "@/lib/artwork";
import { pickImage } from "@/lib/pickFolder";
import { useEditStore } from "@/stores/editStore";
import { toast } from "@/stores/toastStore";

/** Modal metadata editor for tracks and albums. Database-only edits —
 *  file tags are never rewritten. enter saves, esc closes. */
export function MetadataDialog() {
  const target = useEditStore((s) => s.target);
  if (!target) return null;
  return target.kind === "track" ? (
    <TrackForm trackId={target.id} />
  ) : (
    <AlbumForm albumId={target.id} />
  );
}

function TrackForm({ trackId }: { trackId: number }) {
  const close = useEditStore((s) => s.close);
  const queryClient = useQueryClient();
  const { data } = useQuery({
    queryKey: ["track-edit", trackId],
    queryFn: () => api.getTrack(trackId),
  });
  const [form, setForm] = useState<Record<string, string> | null>(null);

  useEffect(() => {
    if (data && !form) {
      setForm({
        title: data.track.title,
        artist: data.artistName,
        album: data.albumName,
        year: data.track.year?.toString() ?? "",
        trackNo: data.track.trackNo?.toString() ?? "",
        discNo: data.track.discNo?.toString() ?? "",
        genre: data.genre ?? "",
      });
    }
  }, [data, form]);

  if (!form) return null;

  const save = async () => {
    try {
      await api.updateTrackMetadata(trackId, {
        title: form.title ?? "",
        artist: form.artist ?? "",
        album: form.album ?? "",
        year: numOrNull(form.year),
        trackNo: numOrNull(form.trackNo),
        discNo: numOrNull(form.discNo),
        genre: form.genre ?? "",
      });
      toast.ok("metadata saved");
      close();
      await queryClient.invalidateQueries();
    } catch (err) {
      toast.error(errMsg(err));
    }
  };

  return (
    <Frame title="edit track" onSave={save}>
      <Field label="title" value={form.title ?? ""} autoFocus onChange={(v) => setForm({ ...form, title: v })} />
      <Field label="artist" value={form.artist ?? ""} onChange={(v) => setForm({ ...form, artist: v })} />
      <Field label="album" value={form.album ?? ""} hint="empty detaches" onChange={(v) => setForm({ ...form, album: v })} />
      <Field label="genre" value={form.genre ?? ""} onChange={(v) => setForm({ ...form, genre: v })} />
      <div className="flex gap-2">
        <Field label="year" value={form.year ?? ""} numeric className="flex-1" onChange={(v) => setForm({ ...form, year: v })} />
        <Field label="track #" value={form.trackNo ?? ""} numeric className="flex-1" onChange={(v) => setForm({ ...form, trackNo: v })} />
        <Field label="disc #" value={form.discNo ?? ""} numeric className="flex-1" onChange={(v) => setForm({ ...form, discNo: v })} />
      </div>
    </Frame>
  );
}

function AlbumForm({ albumId }: { albumId: number }) {
  const close = useEditStore((s) => s.close);
  const queryClient = useQueryClient();
  const { data } = useQuery({
    queryKey: ["album-edit", albumId],
    queryFn: () => api.getAlbum(albumId),
  });
  const [form, setForm] = useState<Record<string, string> | null>(null);
  const [artVersion, setArtVersion] = useState(0);
  const [artError, setArtError] = useState(false);

  const changeArt = async () => {
    const image = await pickImage();
    if (!image) return;
    try {
      await api.setAlbumArtwork(albumId, image);
      setArtVersion((v) => v + 1);
      setArtError(false);
      toast.ok("artwork updated");
      await queryClient.invalidateQueries();
    } catch (err) {
      toast.error(errMsg(err));
    }
  };

  useEffect(() => {
    if (data && !form) {
      setForm({
        name: data.album.name,
        artist: data.album.artistName,
        year: data.album.year?.toString() ?? "",
      });
    }
  }, [data, form]);

  if (!form) return null;

  const save = async () => {
    try {
      const merged = await api.updateAlbumInfo(
        albumId,
        form.name ?? "",
        form.artist ?? "",
        numOrNull(form.year),
      );
      toast.ok(merged ? "merged into existing album" : "album saved");
      close();
      await queryClient.invalidateQueries();
    } catch (err) {
      toast.error(errMsg(err));
    }
  };

  return (
    <Frame title="edit album" onSave={save}>
      <div className="flex items-start gap-3">
        <button
          type="button"
          onClick={() => void changeArt()}
          title="change artwork (.jpg / .png)"
          className="group/art relative h-20 w-20 shrink-0 overflow-hidden border border-subtle bg-base/60 hover:border-focus"
        >
          {data?.album.artworkPath || artVersion > 0 ? (
            !artError ? (
              <img
                src={`${artworkUrl(albumId)}?v=${artVersion}`}
                alt=""
                onError={() => setArtError(true)}
                className="h-full w-full object-cover"
              />
            ) : (
              <span className="flex h-full w-full items-center justify-center text-muted">♪</span>
            )
          ) : (
            <span className="flex h-full w-full items-center justify-center text-muted">♪</span>
          )}
          <span className="absolute inset-0 hidden items-center justify-center bg-black/50 text-[10px] text-primary group-hover/art:flex">
            change
          </span>
        </button>
        <div className="flex min-w-0 flex-1 flex-col gap-2">
          <Field label="name" value={form.name ?? ""} autoFocus onChange={(v) => setForm({ ...form, name: v })} />
          <Field label="artist" value={form.artist ?? ""} onChange={(v) => setForm({ ...form, artist: v })} />
        </div>
      </div>
      <Field label="year" value={form.year ?? ""} numeric className="w-32" onChange={(v) => setForm({ ...form, year: v })} />
    </Frame>
  );
}

function Frame({
  title,
  onSave,
  children,
}: {
  title: string;
  onSave: () => Promise<void>;
  children: React.ReactNode;
}) {
  const close = useEditStore((s) => s.close);
  const ref = useRef<HTMLDivElement>(null);

  return (
    <div
      className="absolute inset-0 z-50 flex items-start justify-center bg-black/40 pt-[18vh]"
      onMouseDown={close}
    >
      <div
        ref={ref}
        className="w-[460px] border border-focus bg-raised"
        onMouseDown={(e) => e.stopPropagation()}
        onKeyDown={(e) => {
          if (e.key === "Escape") {
            e.preventDefault();
            e.stopPropagation();
            close();
          } else if (e.key === "Enter") {
            e.preventDefault();
            void onSave();
          }
        }}
      >
        <header className="flex items-center justify-between border-b border-subtle px-3 py-1.5">
          <span className="text-[11px] text-accent">[ {title} ]</span>
          <span className="text-[10px] text-muted">db only · files untouched</span>
        </header>
        <div className="flex flex-col gap-2 p-3">{children}</div>
        <footer className="flex items-center justify-between border-t border-subtle px-3 py-1.5">
          <span className="text-[10px] text-muted">enter: save · esc: cancel</span>
          <span className="flex gap-2">
            <button
              type="button"
              onClick={close}
              className="border border-subtle px-3 py-0.5 text-[11px] text-muted hover:text-secondary"
            >
              cancel
            </button>
            <button
              type="button"
              onClick={() => void onSave()}
              className="border border-subtle bg-surface px-3 py-0.5 text-[11px] text-accent hover:border-focus"
            >
              save
            </button>
          </span>
        </footer>
      </div>
    </div>
  );
}

function Field({
  label,
  value,
  onChange,
  autoFocus = false,
  numeric = false,
  hint,
  className,
}: {
  label: string;
  value: string;
  onChange: (v: string) => void;
  autoFocus?: boolean;
  numeric?: boolean;
  hint?: string;
  className?: string;
}) {
  return (
    <label className={className}>
      <span className="mb-0.5 flex items-baseline justify-between text-[10px]">
        <span className="text-muted">
          <span className="text-accent">❯</span> {label}
        </span>
        {hint && <span className="text-muted/70">{hint}</span>}
      </span>
      <input
        value={value}
        autoFocus={autoFocus}
        inputMode={numeric ? "numeric" : undefined}
        onChange={(e) =>
          onChange(numeric ? e.target.value.replace(/[^0-9]/g, "") : e.target.value)
        }
        spellCheck={false}
        className="w-full border border-subtle bg-base/60 px-1.5 py-1 text-[12px] text-primary outline-none focus:border-focus"
      />
    </label>
  );
}

function numOrNull(v: string | undefined): number | null {
  const n = Number(v);
  return v && Number.isFinite(n) ? n : null;
}

function errMsg(err: unknown): string {
  return typeof err === "object" && err !== null && "message" in err
    ? String((err as { message: unknown }).message)
    : String(err);
}
