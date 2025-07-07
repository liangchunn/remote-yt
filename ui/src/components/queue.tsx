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
import { useRef } from "react";
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
    return <p className="text-destructive">Server offline</p>;
  }

  return (
    <div>
      <div className="mb-4">
        <h1 className="text-lg font-semibold mb-1 tracking-tight">
          Now Playing
        </h1>
        {nowPlaying && <NowPlaying item={nowPlaying} />}
        {!nowPlaying && !isMutationPending && (
          <p className="text-muted-foreground">Nothing playing</p>
        )}
        {!nowPlaying && isMutationPending && (
          <div className="flex items-center text-muted-foreground">
            <LoaderCircle className="mr-1 h-4 w-4 animate-spin" />{" "}
            <p>Adding to queue...</p>
          </div>
        )}
      </div>
      {(queue.length !== 0 ||
        (queue.length === 0 && !!nowPlaying && isMutationPending)) && (
        <div>
          <h1 className="text-lg font-semibold mb-1 tracking-tight">Up Next</h1>
          <div className="flex flex-col gap-2">
            {queue.map((item) => (
              <QueueListItem key={item.job_id} item={item} />
            ))}
          </div>
          {isMutationPending && (
            <div className="flex items-center text-muted-foreground mt-2">
              <LoaderCircle className="mr-1 h-4 w-4 animate-spin" />{" "}
              <p>Adding to queue...</p>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

function NowPlaying({ item }: { item: QueueItem }) {
  const mutation = useMutation({
    mutationFn: (job_id: string) => {
      return fetch(`/api/cancel/${job_id}`, {
        method: "POST",
      });
    },
  });
  const info = item.track_info;
  return (
    // TODO: when pausing, scale to 95%: "transition-transform scale-95"
    <div
      className={clsx(style.bbbb, "border rounded-md overflow-hidden relative")}
    >
      <img src={info.thumbnail} className=" aspect-video bg-muted" />
      <div className="p-4">
        <h3 className="font-medium text-lg text-center">{info.title}</h3>
        <p className="text-secondary-foreground text-sm text-center">
          {info.channel}
        </p>
        <div className="flex justify-center items-center gap-1 absolute top-1 right-1">
          <Badge label={`${info.width}×${info.height}`} />
          <Codec
            acodec={info.acodec}
            vcodec={info.vcodec}
            track_type={info.track_type}
          />
        </div>
        <div className="flex justify-center items-center">
          <Button variant="ghost" size="icon" className="size-12">
            <SkipBack className="size-5" />
          </Button>
          <Button variant="ghost" size="icon" className="size-12">
            <Rewind className="size-5" />
          </Button>
          <Button variant="ghost" size="icon" className="size-12">
            <Pause className="size-5" />
          </Button>
          <Button variant="ghost" size="icon" className="size-12">
            <FastForward className="size-5" />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            className="size-12"
            onClick={() => mutation.mutate(item.job_id)}
          >
            <SkipForward className="size-5" />
          </Button>
        </div>
      </div>
    </div>
  );
}

function QueueListItem({ item }: { item: QueueItem }) {
  const mutation = useMutation({
    mutationFn: (job_id: string) => {
      return fetch(`/api/cancel/${job_id}`, {
        method: "POST",
      });
    },
  });
  const info = item.track_info;
  return (
    <div className="flex items-center border rounded-md overflow-hidden gap-2">
      <img src={info.thumbnail} className="w-36 self-stretch object-cover" />
      <div className="flex-1 py-2">
        <p className="leading-4">{info.title}</p>
        <p className="text-muted-foreground text-sm">{info.channel}</p>
        <div className="flex  items-center gap-1 top-1 right-1">
          <Badge label={`${info.width}×${info.height}`} />
          <Codec
            acodec={info.acodec}
            vcodec={info.vcodec}
            track_type={info.track_type}
          />
        </div>
      </div>
      <div className="self-start">
        <Button
          variant="ghost"
          size="icon"
          className="size-8"
          onClick={() => mutation.mutate(item.job_id)}
        >
          <X />
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
