import { useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";

type Command =
  | "SeekForward"
  | "SeekRewind"
  | { SeekTo: number }
  | { SetVolume: number }
  | "TogglePause"
  | "Mute"
  | "FullVolume";

export function usePlayerCommandsMutation() {
  const queryClient = useQueryClient();
  const commandMutation = useMutation({
    mutationFn: (command: Command) =>
      (async () => {
        await fetch(`/api/execute_command`, {
          method: "POST",
          body: JSON.stringify(command),
          headers: {
            "Content-Type": "application/json",
          },
        });
        queryClient.invalidateQueries({
          queryKey: ["queue"],
        });
      })(),
  });

  const seekForward = () => commandMutation.mutate("SeekForward");
  const seekRewind = () => commandMutation.mutate("SeekRewind");
  const togglePause = () => commandMutation.mutate("TogglePause");
  const seekTo = (time: number) =>
    commandMutation.mutate({
      SeekTo: time,
    });
  const mute = () => commandMutation.mutate("Mute");
  const fullVolume = () => commandMutation.mutate("FullVolume");
  const setVolume = (percent: number) =>
    commandMutation.mutate({
      SetVolume: percent,
    });

  return {
    seekForward,
    seekRewind,
    togglePause,
    seekTo,
    mute,
    fullVolume,
    setVolume,
  };
}

export function useQueueMutations() {
  const queryClient = useQueryClient();

  const reorderMutation = useMutation({
    mutationFn: ({ job_id, new_pos }: { job_id: string; new_pos: number }) =>
      (async () => {
        await fetch(`/api/move/${job_id}/${new_pos}`, {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
          },
        });
        queryClient.invalidateQueries({
          queryKey: ["queue"],
        });
      })(),
  });

  const cancelMutation = useMutation({
    mutationFn: (job_id: string) => {
      return fetch(`/api/cancel/${job_id}`, {
        method: "POST",
      });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: ["queue"],
      });
    },
  });

  const swapMutation = useMutation({
    mutationFn: (job_id: string) => {
      return fetch(`/api/swap/${job_id}`, {
        method: "POST",
      });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: ["queue"],
      });
    },
  });

  const clearMutation = useMutation({
    mutationFn: () => {
      return fetch(`/api/clear`, {
        method: "POST",
      });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: ["queue"],
      });
    },
  });

  const reorder = (jobId: string, newPos: number) =>
    reorderMutation.mutate({ job_id: jobId, new_pos: newPos });
  const cancel = (jobId: string) => cancelMutation.mutate(jobId);
  const swap = (jobId: string) => swapMutation.mutate(jobId);
  const clear = () => clearMutation.mutate();

  return { reorder, cancel, swap, clear };
}

export function usePlaylistMutations() {
  const queryClient = useQueryClient();

  const queueMutation = useMutation({
    mutationFn: async (playlist_url: string) => {
      const resp = await fetch(`/api/queue_playlist`, {
        method: "POST",
        body: JSON.stringify({
          playlist_url,
        }),
        headers: {
          "Content-Type": "application/json",
        },
      });
      const json = await resp.json();
      if (json.error) {
        throw new Error(json.error);
      }
      return json;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: ["queue"],
      });
    },
    onError: (e) => {
      toast.error(`Failed to queue playlist: ${e.message}`);
    },
  });

  const removeMutation = useMutation({
    mutationFn: async (playlist_url: string) => {
      const resp = await fetch(`/api/remove_playlist`, {
        method: "POST",
        body: JSON.stringify({
          playlist_url,
        }),
        headers: {
          "Content-Type": "application/json",
        },
      });
      if (!resp.ok) {
        const json = await resp.json();
        throw new Error(json.error ?? "failed to remove playlist");
      }
    },
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: ["playlists"],
      });
    },
    onError: (e) => {
      toast.error(`Failed to remove playlist: ${e.message}`);
    },
  });

  const refreshMutation = useMutation({
    mutationFn: async (playlist_url: string) => {
      const resp = await fetch(`/api/refresh_playlist`, {
        method: "POST",
        body: JSON.stringify({
          playlist_url,
        }),
        headers: {
          "Content-Type": "application/json",
        },
      });
      const json = await resp.json();
      if (json.error) {
        throw new Error(json.error);
      }
      return json;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: ["playlists"],
      });
      toast.success("Playlist refreshed");
    },
    onError: (e) => {
      toast.error(`Failed to refresh playlist: ${e.message}`);
    },
  });

  const queueAll = (playlistUrl: string) => queueMutation.mutate(playlistUrl);
  const remove = (playlistUrl: string) => removeMutation.mutate(playlistUrl);
  const refresh = (playlistUrl: string) => refreshMutation.mutate(playlistUrl);

  return { queueAll, remove, refresh };
}

export function useRemoveHistoryEntryMutation() {
  const queryClient = useQueryClient();

  const mutation = useMutation({
    mutationFn: (webpage_url: string) => {
      return fetch(`/api/remove_history`, {
        method: "POST",
        body: JSON.stringify({
          webpage_url,
        }),
        headers: {
          "Content-Type": "application/json",
        },
      });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: ["history"],
      });
    },
  });
  return mutation;
}
