import { usePlayerCommandsMutation } from "@/lib/commands";
import { useMutation } from "@tanstack/react-query";
import {
  SkipBack,
  Rewind,
  Play,
  Pause,
  FastForward,
  SkipForward,
} from "lucide-react";
import { memo } from "react";
import { Button } from "./ui/button";

function PlayerControlsInner({
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

export const PlayerControls = memo(PlayerControlsInner);
