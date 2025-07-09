import type { QueueApi, QueueItem, TrackInfo } from "@/types/queue";
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
} from "lucide-react";
import { useCallback, useRef } from "react";
import style from "./animated-border.module.css";
import clsx from "clsx";

export function Queue({ isMutationPending }: { isMutationPending: boolean }) {
  const { data, error, isSuccess } = useQuery({
    queryKey: ["queue"],
    queryFn: async () => {
      const response = await fetch("/api/inspect");
      if (!response.ok) {
        throw new Error("failed to fetch");
      }
      return (await response.json()) as QueueApi;
    },
    refetchInterval: 1000,
  });

  // https://github.com/TanStack/query/discussions/6910
  const errorRef = useRef(error);
  if (error || isSuccess) errorRef.current = error;

  const items = data ?? [];
  const nowPlaying = items[0] ?? null;
  const queue = items.length > 1 ? items.slice(1) : [];

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
        <NowPlaying item={nowPlaying} isMutationPending={isMutationPending} />
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
}: {
  item: QueueItem | null;
  isMutationPending: boolean;
}) {
  const mutation = useMutation({
    mutationFn: (job_id: string) => {
      return fetch(`/api/cancel/${job_id}`, {
        method: "POST",
      });
    },
  });
  const info = item?.track_info;
  const handleSkip = () => {
    if (item !== null) {
      mutation.mutate(item.job_id);
    }
  };
  return (
    // TODO: when pausing, scale to 95%: "transition-transform scale-95"
    <div
      className={clsx(
        info && [style.bbbb, "border-transparent"],
        !info && "border-muted-background",
        "border-[3px] border-solid rounded-md overflow-hidden relative"
      )}
    >
      {!info && (
        <div className="aspect-video bg-muted/95 flex items-center justify-center">
          {!item && isMutationPending && (
            <LoaderCircle className="h-8 w-8 animate-spin text-muted-foreground" />
          )}
        </div>
      )}
      {info && <img src={info.thumbnail} className="aspect-video bg-muted" />}
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
            <Badge label={`${info.width}×${info.height}`} />
            <Codec
              acodec={info.acodec}
              vcodec={info.vcodec}
              track_type={info.track_type}
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
        <div className="flex justify-center items-center mt-1">
          <Button
            variant="ghost"
            size="icon"
            className="size-12"
            disabled={!item}
          >
            <SkipBack className="size-5" />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            className="size-12"
            disabled={!item}
          >
            <Rewind className="size-5" />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            className="size-12"
            disabled={!item}
          >
            <Pause className="size-5" />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            className="size-12"
            disabled={!item}
          >
            <FastForward className="size-5" />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            className="size-12"
            onClick={handleSkip}
            disabled={!item}
          >
            <SkipForward className="size-5" />
          </Button>
        </div>
      </div>
    </div>
  );
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
              <Badge label={`${info.width}×${info.height}`} />
              <Codec
                acodec={info.acodec}
                vcodec={info.vcodec}
                track_type={info.track_type}
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
