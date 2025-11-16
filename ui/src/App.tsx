import { Queue } from "./components/queue";
import { Form } from "./components/form";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import type { JobType } from "./types/inspect";
import { toast } from "sonner";
import { History } from "./components/history";

const API_MAP: Record<JobType, string> = {
  QueueMerged: "/api/queue_merged",
  QueueSplit: "/api/queue_split",
  Queue: "/api/queue_config",
};

export default function App() {
  const queryClient = useQueryClient();
  const mutation = useMutation({
    mutationFn: async ([job_type, url, min_height]: [
      JobType,
      string,
      number
    ]) => {
      const resp = await fetch(API_MAP[job_type], {
        method: "POST",
        body: JSON.stringify({
          url,
          height: job_type === "Queue" ? undefined : min_height,
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
