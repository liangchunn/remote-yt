import style from "./animated-border.module.css";

import type { InspectItem, PlayerState } from "@/types/inspect";
import clsx from "clsx";
import { LoaderCircle } from "lucide-react";
import { PlayerControls } from "./player-controls";
import { VideoMeta } from "./video-meta";
import { PlayerProgress } from "./player-progress";
import { useEffect } from "react";
import { useQueryClient } from "@tanstack/react-query";

export function NowPlaying({
  item,
  isMutationPending,
  playerState,
}: {
  item: InspectItem | null;
  isMutationPending: boolean;
  playerState: PlayerState | null;
}) {
  const queryClient = useQueryClient();

  const webpageUrl = item?.track_info.webpage_url;
  // reloads the history when the current track changes
  // since the history is updated only after a video is currently playing and is ended
  // either by skipping or played till the end
  useEffect(() => {
    queryClient.invalidateQueries({
      queryKey: ["history"],
    });
  }, [webpageUrl, queryClient]);

  const info = item?.track_info;
  const isGreyBorder = !info || (playerState && playerState.state === "paused");
  return (
    <div>
      <h1 className="text-lg font-semibold mb-1 tracking-tight">Now Playing</h1>

      <div
        className={clsx(
          !isGreyBorder && [style.shiny, "border-transparent"],
          isGreyBorder && "border-muted-background",
          "border-[3px] border-solid rounded-md overflow-hidden relative transition-transform",
          playerState?.state === "paused" ? "scale-99" : "scale-100"
        )}
      >
        {!info && (
          <div className="aspect-video bg-muted/95 flex items-center justify-center">
            {!item && isMutationPending && (
              <LoaderCircle className="h-8 w-8 animate-spin text-muted-foreground" />
            )}
          </div>
        )}
        <div className="relative">
          {info && (
            <img src={info.thumbnail} className="aspect-video bg-muted" />
          )}
          {playerState === null && item && (
            <div className="absolute w-full h-full top-0 left-0 select-none flex items-center justify-center">
              <LoaderCircle className="h-8 w-8 animate-spin text-white/50" />
            </div>
          )}
          <PlayerProgress playerState={playerState} />
        </div>
        <div className="p-4">
          {info && (
            <>
              <h3 className="font-medium text-lg text-center leading-6 mb-1">
                {info.title}
              </h3>
              <p className="text-secondary-foreground text-sm text-center">
                {info.channel}
              </p>
            </>
          )}
          {info && (
            <div className="flex justify-center items-center gap-1 absolute top-1 right-1">
              <VideoMeta
                acodec={info.acodec}
                vcodec={info.vcodec}
                track_type={info.track_type}
                width={info.width}
                height={info.height}
              />
            </div>
          )}
          {!item && (
            <>
              <h3 className="font-medium text-lg text-center text-muted-foreground">
                {isMutationPending ? "Adding to queue..." : "Nothing playing"}
              </h3>
              <p className=" text-sm text-center text-muted-foreground">
                {isMutationPending
                  ? "Just a sec"
                  : "Add something to the queue"}
              </p>
            </>
          )}
          <PlayerControls
            jobId={item?.job_id ?? null}
            playerState={playerState?.state ?? null}
          />
        </div>
      </div>
    </div>
  );
}
