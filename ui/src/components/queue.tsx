import type { QueueApi, QueueItem } from "@/types/queue";
import { useMutation, useQuery } from "@tanstack/react-query";
import { Button } from "./ui/button";
import { X, LoaderCircle, SkipForward, Pause } from "lucide-react";
import { useRef } from "react";

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
        <h1 className="text-lg font-semibold mb-1">Now Playing</h1>
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
          <h1 className="text-lg font-semibold mb-1">Up Next</h1>
          <ul>
            {queue.map((item) => (
              <QueueListItem key={item.job_id} item={item} />
            ))}
            {isMutationPending && (
              <li>
                <div className="flex items-center text-muted-foreground">
                  <LoaderCircle className="mr-1 h-4 w-4 animate-spin" />{" "}
                  <p>Adding to queue...</p>
                </div>
              </li>
            )}
          </ul>
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
  return (
    // TODO: when pausing, scale to 95%: "transition-transform scale-95"
    <div className="border py-4 rounded-md">
      <h3 className="font-medium text-center">{item.title}</h3>
      <p className="text-muted-foreground text-sm text-center">
        {item.channel}
      </p>
      <div className="flex justify-center items-center mt-1">
        <Button variant="ghost" size="icon" disabled>
          <Pause fill="inherit" />
        </Button>
        <Button
          variant="ghost"
          size="icon"
          onClick={() => mutation.mutate(item.job_id)}
        >
          <SkipForward />
        </Button>
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
  return (
    <li className="mb-2">
      <div className="flex items-center">
        <div className="flex-1">
          <p>{item.title}</p>
          <p className="text-muted-foreground text-sm">{item.channel}</p>
        </div>
        <Button
          variant="secondary"
          size="icon"
          className="size-8"
          onClick={() => mutation.mutate(item.job_id)}
        >
          <X />
        </Button>
      </div>
    </li>
  );
}
