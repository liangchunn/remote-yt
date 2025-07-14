import { Queue } from "./components/queue";
import { Form } from "./components/form";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import type { VideoType } from "./types/inspect";
import { toast } from "sonner";
import { History } from "./components/history";

export default function App() {
  const queryClient = useQueryClient();
  const mutation = useMutation({
    mutationFn: async ([video_type, url, min_height]: [
      VideoType,
      string,
      number
    ]) => {
      const fragment = video_type === "merged" ? "queue_merged" : "queue_split";
      const resp = await fetch(`/api/${fragment}`, {
        method: "POST",
        body: JSON.stringify({
          url,
          height: min_height,
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
      toast.error(`Failed to queue: ${e.message}`);
    },
  });
  return (
    <div className="p-4 m-auto max-w-lg pt-4 flex flex-col gap-4 mb-24">
      <Form mutation={mutation} />
      <Queue isMutationPending={mutation.isPending} />
      <History mutation={mutation} />
    </div>
  );
}
