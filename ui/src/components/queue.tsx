import type {
  InspectApi,
  PlayerState,
  QueueItem,
  TrackInfo,
} from "@/types/inspect";
import { useMutation, useQuery } from "@tanstack/react-query";
import { Button } from "./ui/button";
import {
  X,
  LoaderCircle,
  SkipForward,
  Pause,
  SkipBack,
  FastForward,
  Rewind,
  Play,
  Volume2,
  VolumeX,
} from "lucide-react";
import { memo, useCallback, useEffect, useRef, useState } from "react";
import style from "./animated-border.module.css";
import clsx from "clsx";
import { usePlayerCommandsMutation } from "@/lib/commands";

export function Queue({ isMutationPending }: { isMutationPending: boolean }) {
  const { data, error, isSuccess } = useQuery({
    queryKey: ["queue"],
    queryFn: async () => {
      const response = await fetch("/api/inspect");
      if (!response.ok) {
        throw new Error("failed to fetch");
      }
      return (await response.json()) as InspectApi;
    },
    refetchInterval: 1000,
  });

  // https://github.com/TanStack/query/discussions/6910
  const errorRef = useRef(error);
  if (error || isSuccess) errorRef.current = error;

  const items = data?.queue ?? [];
  const nowPlaying = items[0] ?? null;
  const queue = items.length > 1 ? items.slice(1) : [];
  const playerState = data?.player ?? null;

  if (errorRef.current) {
    return (
      <p className="text-destructive text-center text-lg mt-4">
        Server offline
      </p>
    );
  }

  return (
    <div>
      <div className="mb-4">
        <h1 className="text-lg font-semibold mb-1 tracking-tight">
          Now Playing
        </h1>
        <NowPlaying
          item={nowPlaying}
          isMutationPending={isMutationPending}
          playerState={playerState}
        />
      </div>
      {(queue.length !== 0 ||
        (queue.length === 0 && !!nowPlaying && isMutationPending)) && (
        <div>
          <h1 className="text-lg font-semibold mb-1 tracking-tight">Up Next</h1>
          <div className="flex flex-col gap-2">
            {queue.map((item) => (
              <QueueListItem key={item.job_id} item={item} />
            ))}
            {isMutationPending && <QueueListItem item={null} />}
          </div>
        </div>
      )}
    </div>
  );
}

function NowPlaying({
  item,
  isMutationPending,
  playerState,
}: {
  item: QueueItem | null;
  isMutationPending: boolean;
  playerState: PlayerState | null;
}) {
  const info = item?.track_info;
  const isGreyBorder = !info || (playerState && playerState.state === "paused");
  return (
    // TODO: when pausing, scale to 95%: "transition-transform scale-95"
    <div
      className={clsx(
        !isGreyBorder && [style.bbbb, "border-transparent"],
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
        {info && <img src={info.thumbnail} className="aspect-video bg-muted" />}
        {playerState === null && (
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
            <VideoMetaMemoized
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
              Nothing playing
            </h3>
            <p className=" text-sm text-center text-muted-foreground">
              Add something to the queue
            </p>
          </>
        )}
        <PlayerControlsMemoized
          jobId={item?.job_id ?? null}
          playerState={playerState?.state ?? null}
        />
      </div>
    </div>
  );
}

const PlayerControlsMemoized = memo(PlayerControls);

function PlayerControls({
  jobId,
  playerState,
}: {
  jobId: string | null;
  playerState: "playing" | "paused" | null;
}) {
  const mutation = useMutation({
    mutationFn: (job_id: string) => {
      return fetch(`/api/cancel/${job_id}`, {
        method: "POST",
      });
    },
  });
  const handleSkip = () => {
    if (jobId) {
      mutation.mutate(jobId);
    }
  };
  const { seekForward, seekRewind, togglePause } = usePlayerCommandsMutation();

  return (
    <div className="flex justify-center items-center mt-1">
      <Button variant="ghost" size="icon" className="size-12" disabled>
        <SkipBack className="size-5 fill-inherit" />
      </Button>
      <Button
        variant="ghost"
        size="icon"
        className="size-12 cursor-pointer"
        disabled={!jobId || !playerState}
        onClick={seekRewind}
      >
        <Rewind className="size-5 fill-inherit" />
      </Button>
      <Button
        variant="ghost"
        size="icon"
        className="size-12 cursor-pointer"
        disabled={!jobId || !playerState}
        onClick={togglePause}
      >
        {jobId && playerState === "paused" ? (
          <Play className="size-5 fill-inherit" />
        ) : (
          <Pause className="size-5 fill-inherit" />
        )}
      </Button>
      <Button
        variant="ghost"
        size="icon"
        className="size-12 cursor-pointer"
        disabled={!jobId || !playerState}
        onClick={seekForward}
      >
        <FastForward className="size-5 fill-inherit" />
      </Button>
      <Button
        variant="ghost"
        size="icon"
        className="size-12 cursor-pointer"
        onClick={handleSkip}
        disabled={!jobId}
      >
        <SkipForward className="size-5 fill-inherit" />
      </Button>
    </div>
  );
}

function PlayerProgress({ playerState }: { playerState: PlayerState | null }) {
  const [visible, setVisible] = useState(false);

  useEffect(() => {
    if (playerState) {
      setVisible(true);
    } else {
      // Small delay to allow fade-out before hiding
      const timeout = setTimeout(() => setVisible(false), 300);
      return () => clearTimeout(timeout);
    }
  }, [playerState]);

  const [isDragging, setIsDragging] = useState(false);
  const [dragTime, setDragTime] = useState<number | null>(null);
  const barRef = useRef<HTMLDivElement>(null);

  const { seekTo } = usePlayerCommandsMutation();

  const handlePointerDown = (e: React.PointerEvent) => {
    if (!playerState || !barRef.current) return;
    setIsDragging(true);
    barRef.current.setPointerCapture(e.pointerId);
    updateTimeFromPointer(e);
  };

  const handlePointerMove = (e: React.PointerEvent) => {
    if (!isDragging || !playerState || !barRef.current) return;
    updateTimeFromPointer(e);
  };

  const handlePointerUp = (e: React.PointerEvent) => {
    if (!playerState || !barRef.current) return;
    if (isDragging && dragTime !== null) {
      seekTo(dragTime);
    }
    setIsDragging(false);
    setDragTime(null);
    barRef.current.releasePointerCapture(e.pointerId);
  };

  const updateTimeFromPointer = (e: React.PointerEvent) => {
    if (!barRef.current || !playerState) return;
    const rect = barRef.current.getBoundingClientRect();
    const paddingX = 16; // px-4 = 1rem = 16px
    const usableWidth = rect.width - paddingX * 2;
    const offsetX = Math.min(
      Math.max(e.clientX - rect.left - paddingX, 0),
      usableWidth
    );
    const percent = offsetX / usableWidth;
    const newTime = Math.round(percent * playerState.length);
    setDragTime(newTime);
  };

  const currentTime =
    isDragging && dragTime !== null ? dragTime : playerState?.time || 0;
  const progressPercent = (currentTime / (playerState?.length || 1)) * 100;

  const currString = formatTime(
    isDragging && dragTime !== null ? dragTime : playerState?.time || 0
  );
  const totalString = playerState ? formatTime(playerState.length) : "";
  const isMuted = playerState ? playerState.volume === 0 : false;

  return (
    <div
      className={`transition-opacity duration-300 ease-in-out ${
        playerState ? "opacity-100" : "opacity-0"
      } ${visible ? "block" : "hidden"}`}
    >
      <div className="absolute bottom-0 left-0 w-full h-24 bg-gradient-to-t from-black/70 to-black/0" />
      <div className="absolute bottom-6.5 left-0 pl-4">
        <p className="tracking-tight text-sm text-white/50 font-mono">
          <span>{currString}</span>
          <span className="mx-0.5">/</span>
          <span>{totalString}</span>
        </p>
      </div>
      <div className="absolute bottom-6.5 right-0 pr-4">
        <MutedButtonMemoized isMuted={isMuted} />
      </div>
      <div
        className="absolute bottom-1.5 left-0 w-full h-6 px-4 cursor-pointer touch-none"
        ref={barRef}
        onPointerDown={handlePointerDown}
        onPointerMove={handlePointerMove}
        onPointerUp={handlePointerUp}
      >
        <div className="relative h-full">
          <div className="bg-white/50 h-1 absolute left-0 top-1/2 -translate-y-1/2 w-full rounded-full"></div>
          <div
            className="bg-red-500 h-1 absolute left-0 top-1/2 -translate-y-1/2 rounded-full"
            style={{ width: `${progressPercent}%` }}
          ></div>
          <div
            className="absolute top-1/2 -translate-y-1/2 w-3 h-3 bg-red-500 rounded-full shadow"
            style={{
              left: `calc(${progressPercent}% - 6px)`, // 6px = half of 12px width
              transition: isDragging ? "none" : "left 0.1s linear",
              zIndex: 10,
            }}
          />
        </div>
      </div>
    </div>
  );
}

const MutedButtonMemoized = memo(MuteButton);

function MuteButton({ isMuted }: { isMuted: boolean }) {
  const { fullVolume, mute } = usePlayerCommandsMutation();
  const handleVolume = () => {
    if (isMuted) {
      fullVolume();
    } else {
      mute();
    }
  };
  return (
    <Button
      variant="ghost"
      className="cursor-pointer hover:bg-muted/15 size-6"
      onClick={handleVolume}
    >
      {!isMuted && <Volume2 className="stroke-white/80" />}
      {isMuted && <VolumeX className="stroke-white/80" />}
    </Button>
  );
}

function formatTime(seconds: number) {
  const hrs = Math.floor(seconds / 3600);
  const mins = Math.floor((seconds % 3600) / 60);
  const secs = seconds % 60;

  if (hrs > 0) {
    return `${hrs}:${mins.toString().padStart(2, "0")}:${secs
      .toString()
      .padStart(2, "0")}`;
  } else {
    return `${mins}:${secs.toString().padStart(2, "0")}`;
  }
}

function QueueListItem({ item }: { item: QueueItem | null }) {
  const mutation = useMutation({
    mutationFn: (job_id: string) => {
      return fetch(`/api/cancel/${job_id}`, {
        method: "POST",
      });
    },
  });
  const info = item?.track_info;
  const handleUnqueue = useCallback(() => {
    if (info) {
      mutation.mutate(item.job_id);
    }
  }, [info, item, mutation]);
  const isRemoving = mutation.isPending;
  return (
    <div className="flex items-center border rounded-md overflow-hidden gap-2">
      {info ? (
        <img
          src={info.thumbnail}
          className="w-36 self-stretch object-cover bg-muted"
        />
      ) : (
        <div className="w-36 self-stretch object-cover bg-muted/95 ">
          <div className="aspect-video flex items-center justify-center">
            <LoaderCircle className="h-6 w-6 animate-spin text-muted-foreground" />
          </div>
        </div>
      )}
      <div className="flex-1 py-3 pl-1">
        {info ? (
          <>
            <p className="leading-4 mb-0.5">{info.title}</p>
            <p className="text-muted-foreground text-sm mb-1">{info.channel}</p>
            <div className="flex items-center gap-1 top-1 right-1">
              <VideoMetaMemoized
                acodec={info.acodec}
                vcodec={info.vcodec}
                track_type={info.track_type}
                width={info.width}
                height={info.height}
              />
            </div>
          </>
        ) : (
          <p className="text-muted-foreground">Adding to queue...</p>
        )}
      </div>
      <div className="self-start">
        <Button
          variant="ghost"
          size="icon"
          className="size-8"
          onClick={handleUnqueue}
          disabled={!item || isRemoving}
        >
          {isRemoving && (
            <LoaderCircle className="animate-spin text-muted-foreground" />
          )}
          {!isRemoving && <X />}
        </Button>
      </div>
    </div>
  );
}

function trimFormat(codec: string) {
  if (codec.includes(".")) {
    return codec.split(".")[0];
  } else {
    return codec;
  }
}

const VideoMetaMemoized = memo(VideoMeta);

function VideoMeta({
  acodec,
  vcodec,
  track_type,
  width,
  height,
}: Pick<TrackInfo, "acodec" | "vcodec" | "track_type" | "width" | "height">) {
  return (
    <>
      <Badge label={`${width}×${height}`} />
      <Codec acodec={acodec} track_type={track_type} vcodec={vcodec} />
    </>
  );
}

function Codec({
  acodec,
  vcodec,
  track_type,
}: Pick<TrackInfo, "acodec" | "vcodec" | "track_type">) {
  if (track_type === "merged") {
    return <Badge label={`${trimFormat(vcodec)}+${trimFormat(acodec)}`} />;
  }
  if (track_type === "split") {
    return (
      <>
        <Badge label={trimFormat(vcodec)} />
        <Badge label={trimFormat(acodec)} />
      </>
    );
  }
}

function Badge({ label }: { label: string }) {
  return (
    <div className="text-xs font-mono py-0.5 px-1 border rounded-sm inline-block text-secondary-foreground bg-background/75">
      {label}
    </div>
  );
}
