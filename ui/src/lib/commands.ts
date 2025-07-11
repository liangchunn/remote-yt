import { useMutation, useQueryClient } from "@tanstack/react-query";

type Command =
  | "SeekForward"
  | "SeekRewind"
  | { SeekTo: number }
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

  return {
    seekForward,
    seekRewind,
    togglePause,
    seekTo,
    mute,
    fullVolume,
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

  const reorder = (jobId: string, newPos: number) =>
    reorderMutation.mutate({ job_id: jobId, new_pos: newPos });

  return { reorder };
}
