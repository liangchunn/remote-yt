import { useMutation, useQueryClient } from "@tanstack/react-query";

type Command =
  | "SeekForward"
  | "SeekRewind"
  | { SeekTo: number }
  | "TogglePause";

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
  const togglePause = async () => commandMutation.mutate("TogglePause");

  return {
    seekForward,
    seekRewind,
    togglePause,
  };
}
