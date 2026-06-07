import type { PlaylistEntry } from "@/types/inspect";
import { useQuery } from "@tanstack/react-query";
import { Copy, EllipsisIcon, ListEnd, RefreshCw, Trash } from "lucide-react";
import { usePlaylistMutations } from "@/lib/commands";
import { copyUrlToClipboard } from "@/lib/copy-url";
import { Button } from "./ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "./ui/dropdown-menu";
import { SafeImage } from "./safe-image";
import { formatTime } from "@/lib/format-time";

export function SavedPlaylists() {
  const { data } = useQuery({
    queryKey: ["playlists"],
    queryFn: () =>
      fetch("/api/playlists")
        .then((res) => res.json())
        .then((data) => data as PlaylistEntry[]),
  });

  if (!data || data.length === 0) {
    return null;
  }

  return (
    <div>
      <h1 className="text-lg font-semibold mb-1 tracking-tight">Playlists</h1>
      <div className="flex flex-col gap-2">
        {data.map((playlist) => (
          <PlaylistItem key={playlist.id} playlist={playlist} />
        ))}
      </div>
    </div>
  );
}

function PlaylistItem({ playlist }: { playlist: PlaylistEntry }) {
  const { queueAll, refresh, remove } = usePlaylistMutations();

  return (
    <div className="flex items-center border rounded-md overflow-hidden gap-2 bg-card text-card-foreground select-none">
      <div className="w-36 min-h-20 self-stretch relative flex bg-muted">
        <SafeImage
          src={playlist.thumbnail}
          className="h-full w-full object-cover bg-muted"
        />
      </div>
      <div className="flex-1 py-3 pl-1">
        <p className="leading-5 mb-0.5 line-clamp-2">{playlist.title}</p>
        <p className="text-muted-foreground text-sm mb-0.5">
          {playlist.video_count}{" "}
          {playlist.video_count === 1 ? "video" : "videos"} ·{" "}
          {formatTime(playlist.total_duration)}
        </p>
      </div>
      <div className="self-start">
        <DropdownMenu>
          <DropdownMenuTrigger
            render={
              <Button variant="ghost" size="icon" className="size-8">
                <EllipsisIcon />
              </Button>
            }
          />
          <DropdownMenuContent align="end" className="w-max min-w-max">
            <DropdownMenuItem
              className="whitespace-nowrap"
              onClick={() => queueAll(playlist.webpage_url)}
            >
              <ListEnd className="mr-1" />
              Queue all
            </DropdownMenuItem>
            <DropdownMenuItem
              className="whitespace-nowrap"
              onClick={() => copyUrlToClipboard(playlist.webpage_url)}
            >
              <Copy className="mr-1" />
              Copy URL
            </DropdownMenuItem>
            <DropdownMenuItem
              className="whitespace-nowrap"
              onClick={() => refresh(playlist.webpage_url)}
            >
              <RefreshCw className="mr-1" />
              Refresh playlist
            </DropdownMenuItem>
            <DropdownMenuItem
              variant="destructive"
              className="whitespace-nowrap"
              onClick={() => remove(playlist.webpage_url)}
            >
              <Trash className="mr-1" />
              Remove entry
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
    </div>
  );
}
